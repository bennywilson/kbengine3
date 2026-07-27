#!/usr/bin/env python3
"""Converts the engine's exported OpenVLA training tuples (PNG frames +
`manifest.jsonl`, written by the "Export Training Data" / "Batch Export..."
buttons in the Policy Control panel -- see `src/policy_dataset.rs` and
`examples/splat/src/example_game.rs`) into an RLDS-formatted TFDS dataset,
the input format OpenVLA's own `finetune.py` (and Octo's) expect.

This is a one-off, offline step -- nothing in the Rust engine reads or writes
TFDS/RLDS itself. Run it once you have as many `resources/openvla_datasets/
demo_*/` folders exported as you want in the fine-tune, then point
`finetune.py`'s `--data_root_dir` (or an OXE mixture entry in
`prismatic/vla/datasets/rlds/oxe/configs.py`) at the output directory.

Usage:
    python3 tools/build_rlds_dataset.py \
        --data-dir examples/splat/resources/openvla_datasets \
        --output-dir ~/tensorflow_datasets

Requires: tensorflow, tensorflow-datasets, pillow, numpy (see
`tools/requirements-rlds.txt` -- deliberately kept out of the main Rust
project's own deps since these are heavy and only needed for this one
offline conversion step, not for running the engine or its inference
pipeline).

Dataset shape follows the widely-used "example_dataset" RLDS template
(https://github.com/kpertsch/rlds_dataset_builder), the same layout the
Open X-Embodiment datasets (Bridge, RT-1, etc.) and OpenVLA's own
fine-tuning configs already expect: one `steps` sequence per episode, each
step carrying `observation/image`, `action`, `language_instruction`,
`discount`, `reward`, `is_first`, `is_last`, `is_terminal`.

Source of truth per episode: each `demo_*/manifest.jsonl` line is already
exactly one step (`{"image","instruction","action"}`, in playback order) --
this script does no re-derivation of the action math, it only repackages
what `policy_dataset.rs` already computed.
"""
import argparse
import json
import pathlib
import shutil
import sys

try:
    import numpy as np
    import tensorflow_datasets as tfds
    from PIL import Image
except ImportError:
    sys.exit(
        "This script needs tensorflow-datasets, pillow and numpy: "
        "pip install -r tools/requirements-rlds.txt"
    )

# All exports so far come from the same fixed policy camera + capture-width
# calc (example_game.rs's `policy_capture_dims`), so every frame is this
# exact size. If a future export uses a different camera/aspect, bump this
# (and the dataset VERSION below) rather than trying to support mixed sizes
# in one dataset.
IMAGE_SHAPE = (224, 366, 3)

# Temporal subsampling factor. The engine exports one tuple per simulation
# frame at the source demos' native 20Hz (dt=0.05), which makes each action a
# ~2-3mm translation delta -- so small it's near the noise floor and close to
# unlearnable from a single static frame (the first fine-tune attempt collapsed
# to emitting one constant action and ignoring the image entirely). Keeping
# every Nth frame and accumulating the actions in between yields proportionally
# larger, better-conditioned targets at a control rate closer to what the base
# model saw in Bridge/OXE, and shrinks an epoch by the same factor.
DEFAULT_STRIDE = 4


def _axis_angle_to_quat(axis_angle: np.ndarray) -> np.ndarray:
    """Exponential map: rotation vector -> quaternion (w, x, y, z)."""
    angle = float(np.linalg.norm(axis_angle))
    if angle < 1e-12:
        return np.array([1.0, 0.0, 0.0, 0.0])
    axis = axis_angle / angle
    half = angle / 2.0
    return np.concatenate([[np.cos(half)], axis * np.sin(half)])


def _quat_mul(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """Hamilton product of two (w, x, y, z) quaternions."""
    aw, ax, ay, az = a
    bw, bx, by, bz = b
    return np.array(
        [
            aw * bw - ax * bx - ay * by - az * bz,
            aw * bx + ax * bw + ay * bz - az * by,
            aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw,
        ]
    )


def _quat_to_axis_angle(q: np.ndarray) -> np.ndarray:
    """Log map: quaternion -> rotation vector, canonicalized to w >= 0.

    The w >= 0 flip matters: a quaternion and its negation describe the same
    orientation, so without it a small rotation can come back as its ~2*pi
    complement (the same sign convention `src/policy_dataset.rs` applies when
    producing these deltas in the first place).
    """
    q = q / np.linalg.norm(q)
    if q[0] < 0.0:
        q = -q
    vec_norm = float(np.linalg.norm(q[1:]))
    if vec_norm < 1e-12:
        return np.zeros(3)
    angle = 2.0 * np.arctan2(vec_norm, float(q[0]))
    return (q[1:] / vec_norm) * angle


def _accumulate_actions(actions) -> np.ndarray:
    """Composes consecutive single-frame actions into one strided action.

    Translation deltas are world-frame, so they simply add. Rotations do not:
    each delta is `q_next * conj(q_curr)` (world-frame extrinsic composition,
    see `MujocoScene::apply_policy_action`), so successive deltas compose by
    left-multiplication in time order, and the result is re-expressed as a
    rotation vector. Summing the rotation vectors directly would only be a
    small-angle approximation. The gripper channel is an absolute target
    (0 = open, 1 = closed) rather than a delta, so it takes the window's final
    value -- the state the gripper should have reached by the end of the window.
    """
    translation = np.sum([a[:3] for a in actions], axis=0)
    rotation = np.array([1.0, 0.0, 0.0, 0.0])
    for a in actions:
        rotation = _quat_mul(_axis_angle_to_quat(np.asarray(a[3:6], dtype=np.float64)), rotation)
    return np.concatenate([translation, _quat_to_axis_angle(rotation), [actions[-1][6]]]).astype(np.float32)


def _load_episode(manifest_path: pathlib.Path, stride: int = 1):
    """Reads one demo_*/manifest.jsonl into a list of step dicts.

    With `stride > 1`, keeps every Nth frame's image and pairs it with the
    accumulated action over the N frames that follow (see
    `_accumulate_actions`). A trailing partial window is still emitted rather
    than dropped -- it's a valid, just-shorter, action -- but a window with no
    frames at all is not.
    """
    rows = [json.loads(line) for line in manifest_path.read_text().splitlines() if line.strip()]
    demo_dir = manifest_path.parent
    steps = []
    for start in range(0, len(rows), stride):
        window = rows[start : start + stride]
        if not window:
            break
        image = Image.open(demo_dir / window[0]["image"]).convert("RGB")
        if image.size != (IMAGE_SHAPE[1], IMAGE_SHAPE[0]):
            raise ValueError(
                f"{demo_dir / window[0]['image']}: expected size "
                f"{(IMAGE_SHAPE[1], IMAGE_SHAPE[0])}, got {image.size}"
            )
        steps.append(
            {
                "image": np.array(image, dtype=np.uint8),
                "action": _accumulate_actions([row["action"] for row in window]),
                "instruction": window[0]["instruction"],
            }
        )
    return steps


class BlackSplatToolHang(tfds.core.GeneratorBasedBuilder):
    """RLDS export of this project's MuJoCo/wgpu-rendered ToolHang demos."""

    # The stride is baked into the version rather than exposed as a TFDS
    # BuilderConfig: a config would change the dataset's name to
    # "black_splat_tool_hang/<config>", which no longer matches the plain key
    # registered in OpenVLA's `OXE_DATASET_CONFIGS`/`OXE_STANDARDIZATION_TRANSFORMS`.
    # `tfds.builder(name, data_dir=...)` resolves to the highest version present,
    # so a new build supersedes the old one while leaving it on disk. Bump this
    # when changing `--stride` (or anything else about the action encoding), or
    # the two builds collide in the same directory.
    VERSION = tfds.core.Version("2.0.0")
    RELEASE_NOTES = {
        "1.0.0": "Initial release. One tuple per 20Hz simulation frame (stride 1).",
        "2.0.0": "Stride 4 (~5Hz): actions accumulated over 4 frames, see DEFAULT_STRIDE.",
    }

    def __init__(self, *, source_dir, stride=DEFAULT_STRIDE, holdout=frozenset(), **kwargs):
        self.source_dir = pathlib.Path(source_dir)
        self.stride = stride
        self.holdout = holdout
        super().__init__(**kwargs)

    def _info(self) -> tfds.core.DatasetInfo:
        return tfds.core.DatasetInfo(
            builder=self,
            description=__doc__,
            features=tfds.features.FeaturesDict(
                {
                    "steps": tfds.features.Dataset(
                        {
                            "observation": tfds.features.FeaturesDict(
                                {
                                    "image": tfds.features.Image(
                                        shape=IMAGE_SHAPE,
                                        dtype=np.uint8,
                                        encoding_format="png",
                                        doc="Fixed policy camera capture, RGB.",
                                    ),
                                }
                            ),
                            "action": tfds.features.Tensor(
                                shape=(7,),
                                dtype=np.float32,
                                doc="[dx,dy,dz,drx,dry,drz,gripper], world-frame "
                                "extrinsic rotation delta, gripper 0=open/1=closed "
                                "-- matches MujocoScene::apply_policy_action exactly.",
                            ),
                            "discount": tfds.features.Scalar(
                                dtype=np.float32, doc="Discount, always 1.0 (undiscounted demo)."
                            ),
                            "reward": tfds.features.Scalar(
                                dtype=np.float32, doc="Sparse: 1.0 on the final step, else 0.0."
                            ),
                            "is_first": tfds.features.Scalar(dtype=np.bool_),
                            "is_last": tfds.features.Scalar(dtype=np.bool_),
                            "is_terminal": tfds.features.Scalar(dtype=np.bool_),
                            "language_instruction": tfds.features.Text(),
                        }
                    ),
                    "episode_metadata": tfds.features.FeaturesDict(
                        {
                            "file_path": tfds.features.Text(
                                doc="Source demo_*/ folder this episode came from."
                            ),
                        }
                    ),
                }
            ),
            supervised_keys=None,
            homepage="https://github.com/kpertsch/rlds_dataset_builder",
        )

    def _split_generators(self, dl_manager):
        del dl_manager
        return {"train": self._generate_examples(self.source_dir)}

    def _generate_examples(self, path: pathlib.Path):
        for manifest_path in sorted(path.glob("demo_*/manifest.jsonl")):
            demo_name = manifest_path.parent.name
            if demo_name in self.holdout:
                continue
            steps = _load_episode(manifest_path, self.stride)
            if not steps:
                continue
            episode = []
            last = len(steps) - 1
            for i, step in enumerate(steps):
                episode.append(
                    {
                        "observation": {"image": step["image"]},
                        "action": step["action"],
                        "discount": 1.0,
                        "reward": 1.0 if i == last else 0.0,
                        "is_first": i == 0,
                        "is_last": i == last,
                        "is_terminal": i == last,
                        "language_instruction": step["instruction"],
                    }
                )
            yield demo_name, {
                "steps": episode,
                "episode_metadata": {"file_path": str(manifest_path.parent)},
            }


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument(
        "--data-dir",
        default="examples/splat/resources/openvla_datasets",
        help="Directory containing demo_*/manifest.jsonl exports (default: %(default)s).",
    )
    parser.add_argument(
        "--output-dir",
        default=None,
        help="TFDS data_dir to write into (default: TFDS's own default, ~/tensorflow_datasets).",
    )
    parser.add_argument(
        "--stride",
        type=int,
        default=DEFAULT_STRIDE,
        help="Keep every Nth exported frame, accumulating the actions in between "
        "(default: %(default)s). Bump the builder's VERSION when changing this.",
    )
    parser.add_argument(
        "--holdout",
        default="",
        help="Comma-separated demo folder names (e.g. demo_3,demo_17) to exclude from "
        "training and reserve for tools/eval_openvla.py -- the model never sees these, "
        "so evaluating against them measures generalization rather than memorization. "
        "Written to <data-dir>/holdout.json every run (even empty) so eval_openvla.py "
        "always reads the current truth rather than a stale list from an earlier build.",
    )
    args = parser.parse_args()

    if args.stride < 1:
        sys.exit(f"--stride must be >= 1, got {args.stride}")
    source_dir = pathlib.Path(args.data_dir).resolve()
    if not source_dir.is_dir():
        sys.exit(f"No such directory: {source_dir}")
    all_demos = {p.parent.name for p in source_dir.glob("demo_*/manifest.jsonl")}
    if not all_demos:
        sys.exit(f"No demo_*/manifest.jsonl found under {source_dir}")

    holdout = {name.strip() for name in args.holdout.split(",") if name.strip()}
    unknown = holdout - all_demos
    if unknown:
        sys.exit(f"--holdout names not found under {source_dir}: {', '.join(sorted(unknown))}")
    (source_dir / "holdout.json").write_text(json.dumps(sorted(holdout)))

    n_demos = len(all_demos) - len(holdout)
    if n_demos == 0:
        sys.exit(f"--holdout excludes every demo under {source_dir} -- nothing left to train on")
    print(
        f"==> Building RLDS dataset from {n_demos} demo(s) in {source_dir} (stride {args.stride})"
        + (f", holding out {len(holdout)}: {', '.join(sorted(holdout))}" if holdout else "")
    )

    # TFDS keys its cache by (name, version, data_dir) and silently reuses
    # whatever's already there -- rerunning after re-exporting demos, or
    # changing --holdout, would otherwise train on stale data with no
    # warning. Force a real rebuild every time instead.
    output_root = pathlib.Path(args.output_dir).expanduser() if args.output_dir else pathlib.Path.home() / "tensorflow_datasets"
    stale = output_root / "black_splat_tool_hang" / str(BlackSplatToolHang.VERSION)
    if stale.exists():
        shutil.rmtree(stale)

    builder = BlackSplatToolHang(source_dir=source_dir, stride=args.stride, holdout=holdout, data_dir=args.output_dir)
    builder.download_and_prepare()
    print(f"==> Done. Dataset written to {builder.data_path}")


if __name__ == "__main__":
    main()
