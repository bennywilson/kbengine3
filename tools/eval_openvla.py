#!/usr/bin/env python3
"""Evaluates a fine-tuned OpenVLA LoRA adapter against held-out demos --
i.e. whether it generalizes past its training set, not just whether the
training loss went down.

Reads exactly the demos listed in <data-dir>/holdout.json (written by
build_rlds_dataset.py's --holdout flag) -- these were excluded from that
TFDS build, so the checkpoint being evaluated has never seen them. Runs each
held-out step through the model and the same accumulated-action math
build_rlds_dataset.py trains on (imported from there directly, so the two
can't drift apart), then reports:

  - Pearson r between predicted and ground-truth actions per dimension
    (higher = better; the model is tracking real image signal, not just
    memorizing/guessing).
  - Mean absolute error, model vs. a dumb constant-mean baseline, so
    "N.Nx better than baseline" has one precise, reproducible meaning:
    baseline MAE / model MAE.

Usage:
    python tools/eval_openvla.py \\
        --adapter-dir D:\\openvla_runs\\<run>\\adapter-tmp\\<exp_id> \\
        --data-dir examples/splat/resources/openvla_datasets/<scene>

Requires the training venv (torch, transformers, peft) -- see
tools/retrain_openvla.bat's OPENVLA_VENV, not tools/requirements-rlds.txt
(no tensorflow/tfds needed here at all; this reads the raw manifest.jsonl
exports directly, the same source build_rlds_dataset.py itself reads).
"""
import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_rlds_dataset import DEFAULT_STRIDE, _load_episode  # noqa: E402  (reuse training's own action math)

try:
    import numpy as np
    import torch
    from peft import PeftModel
    from PIL import Image
    from transformers import AutoModelForVision2Seq, AutoProcessor, BitsAndBytesConfig
except ImportError:
    sys.exit(
        "This script needs torch, transformers, peft, pillow, numpy -- the openvla "
        "training venv (see tools/retrain_openvla.bat's OPENVLA_VENV), not "
        "tools/requirements-rlds.txt."
    )

ACTION_DIMS = ["dx", "dy", "dz", "drx", "dry", "drz", "gripper"]


def load_holdout_steps(data_dir: Path, stride: int):
    holdout_path = data_dir / "holdout.json"
    if not holdout_path.is_file():
        sys.exit(
            f"No {holdout_path} -- rebuild the TFDS dataset with "
            f"--holdout demo_X,demo_Y first (see build_rlds_dataset.py). Without a "
            f"recorded holdout set there's no unseen data to evaluate against."
        )
    names = json.loads(holdout_path.read_text())
    if not names:
        sys.exit(f"{holdout_path} lists zero held-out demos -- nothing to evaluate.")
    steps = []
    for name in names:
        manifest = data_dir / name / "manifest.jsonl"
        if not manifest.is_file():
            sys.exit(f"Held-out demo '{name}' has no manifest.jsonl under {data_dir} -- was it exported?")
        steps.extend(_load_episode(manifest, stride))
    return names, steps


def load_model(adapter_dir: Path, vla_path: str, dataset_stats: Path):
    device = "cuda" if torch.cuda.is_available() else "cpu"
    if device == "cpu":
        print("WARNING: no CUDA device found, this will be extremely slow.")
    processor = AutoProcessor.from_pretrained(vla_path, trust_remote_code=True)

    # Same NF4 4-bit config as policy_server/openvla/server.py's ADAPTER_DIR
    # path -- matches how these checkpoints were actually trained (QLoRA),
    # and bf16 doesn't leave workable headroom on a 16GB card anyway.
    quantization_config = None
    if device == "cuda":
        quantization_config = BitsAndBytesConfig(
            load_in_4bit=True, bnb_4bit_compute_dtype=torch.bfloat16, bnb_4bit_quant_type="nf4"
        )
    model = AutoModelForVision2Seq.from_pretrained(
        vla_path,
        attn_implementation="eager",
        torch_dtype=torch.bfloat16 if device == "cuda" else torch.float32,
        quantization_config=quantization_config,
        low_cpu_mem_usage=True,
        trust_remote_code=True,
    )
    if quantization_config is None:
        model = model.to(device)

    print(f"applying LoRA adapter: {adapter_dir}")
    model = PeftModel.from_pretrained(model, str(adapter_dir)).base_model.model
    model.eval()

    if not dataset_stats.is_file():
        sys.exit(
            f"No dataset_statistics.json at {dataset_stats} -- pass --dataset-stats "
            f"explicitly if your run doesn't follow retrain_openvla.bat's layout "
            f"(<run_dir>/adapter-tmp/<exp_id> for the adapter, <run_dir>/<exp_id>/"
            f"dataset_statistics.json for stats)."
        )
    with open(dataset_stats) as f:
        model.norm_stats = json.load(f)
    return model, processor, device


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--adapter-dir", required=True, help="LoRA checkpoint dir (adapter_config.json + adapter_model.safetensors).")
    parser.add_argument(
        "--data-dir",
        default="examples/splat/resources/openvla_datasets",
        help="Scene export dir containing demo_*/ and holdout.json (default: %(default)s).",
    )
    parser.add_argument("--vla-path", default="openvla/openvla-7b")
    parser.add_argument("--unnorm-key", default="black_splat_tool_hang")
    parser.add_argument(
        "--dataset-stats",
        default=None,
        help="Defaults to <run_dir>/<exp_id>/dataset_statistics.json, derived from "
        "--adapter-dir assuming retrain_openvla.bat's <run_dir>/adapter-tmp/<exp_id> layout.",
    )
    parser.add_argument(
        "--stride",
        type=int,
        default=DEFAULT_STRIDE,
        help="Must match the stride the checkpoint was actually trained with "
        "(default: %(default)s) -- a mismatch compares apples to oranges.",
    )
    args = parser.parse_args()

    adapter_dir = Path(args.adapter_dir).resolve()
    if not (adapter_dir / "adapter_config.json").is_file():
        sys.exit(f"{adapter_dir} doesn't look like an adapter checkpoint (no adapter_config.json).")
    data_dir = Path(args.data_dir).resolve()
    dataset_stats = (
        Path(args.dataset_stats)
        if args.dataset_stats
        else adapter_dir.parent.parent / adapter_dir.name / "dataset_statistics.json"
    )

    names, steps = load_holdout_steps(data_dir, args.stride)
    print(f"==> Evaluating on {len(steps)} step(s) from {len(names)} held-out demo(s): {', '.join(names)}")

    model, processor, device = load_model(adapter_dir, args.vla_path, dataset_stats)

    preds, truths = [], []
    with torch.inference_mode():
        for i, step in enumerate(steps):
            image = Image.fromarray(step["image"])
            prompt = f"In: What action should the robot take to {step['instruction']}?\nOut:"
            inputs = processor(prompt, image).to(device, dtype=torch.bfloat16 if device == "cuda" else torch.float32)
            # Same drop as server.py's /act handler -- predict_action() can append a
            # marker token to input_ids without extending attention_mask to match.
            inputs.pop("attention_mask", None)
            action = model.predict_action(**inputs, unnorm_key=args.unnorm_key, do_sample=False)
            preds.append(np.asarray(action, dtype=np.float64))
            truths.append(np.asarray(step["action"], dtype=np.float64))
            if (i + 1) % 10 == 0 or (i + 1) == len(steps):
                print(f"  ...{i + 1}/{len(steps)}")

    preds = np.stack(preds)
    truths = np.stack(truths)
    baseline = np.tile(truths.mean(axis=0), (len(truths), 1))

    print(f"\n{'dim':<8}{'r':>8}{'model MAE':>12}{'baseline MAE':>14}{'better':>10}")
    model_mae_all, baseline_mae_all = [], []
    for d, name in enumerate(ACTION_DIMS):
        # Correlation is undefined (not just zero) against a constant series --
        # report NaN honestly rather than a misleading number.
        r = float("nan") if np.std(truths[:, d]) < 1e-9 else float(np.corrcoef(preds[:, d], truths[:, d])[0, 1])
        model_mae = float(np.mean(np.abs(preds[:, d] - truths[:, d])))
        baseline_mae = float(np.mean(np.abs(baseline[:, d] - truths[:, d])))
        model_mae_all.append(model_mae)
        baseline_mae_all.append(baseline_mae)
        ratio = baseline_mae / model_mae if model_mae > 1e-12 else float("inf")
        print(f"{name:<8}{r:>8.2f}{model_mae:>12.4f}{baseline_mae:>14.4f}{ratio:>9.2f}x")

    overall_ratio = float(np.mean(baseline_mae_all) / np.mean(model_mae_all))
    print(
        f"\n==> Overall: {overall_ratio:.2f}x better than constant-baseline MAE, "
        f"on {len(steps)} never-trained-on step(s) from {len(names)} held-out demo(s)."
    )


if __name__ == "__main__":
    main()
