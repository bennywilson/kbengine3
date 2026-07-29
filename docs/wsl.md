# WSL in this project

WSL is not a general dev environment here — nothing in the engine, the editor, or
the Rust build touches it. It exists for exactly three narrow jobs in the OpenVLA
policy pipeline:

1. **Building the RLDS/TFDS dataset**, because native Windows has no working
   tensorflow + tensorflow-datasets install.
2. **Storing that dataset** where the native Windows trainer copies it from.
3. **Backing Docker Desktop**, which is how the GPU policy server runs.

Everything else in the pipeline — demo conversion, fine-tuning, eval, the stub
server — is native Windows Python.

## At a glance

| Step | Entry point | Where it runs | WSL's role |
| --- | --- | --- | --- |
| Engine / editor | `cargo run --release` in `examples/splat/` | native | none |
| Convert robomimic demos | `tools/convert_robomimic_demos.bat` | native Python (`h5py`) | none |
| **Rebuild TFDS dataset** | `tools/rebuild_rlds_dataset.bat` | **inside WSL** | runs the entire build |
| Fine-tune OpenVLA | `tools/retrain_openvla.bat` | native Windows | source of the dataset it mirrors |
| Run eval | `tools/eval_openvla.bat` | native Windows | none — reads `manifest.jsonl`, not TFDS |
| Policy server (stub) | `policy_server/run_server.bat stub` | native Python | none |
| Policy server (openvla) | `policy_server/run_server.bat openvla` | Docker | WSL2 is Docker's VM + CUDA passthrough |

The editor's own buttons (Training Data Export panel) call the three `.bat` files
above, so the same split applies when driving the pipeline from the UI.

## 1. Building the dataset — the only step that *runs* in WSL

[tools/rebuild_rlds_dataset.bat](../tools/rebuild_rlds_dataset.bat) shells out to
`wsl -d Ubuntu` and runs [tools/build_rlds_dataset.py](../tools/build_rlds_dataset.py)
there.

**Why:** the blocker is a single Linux-only dependency, `array_record` — the
storage backend TFDS uses for the RLDS record format. tensorflow-datasets declares
it with an environment marker:

```
Requires-Dist: array_record>=0.5.0; platform_system == "Linux"
```

On Windows that marker is false, so **pip skips the dependency entirely and reports
a successful install**. Nothing errors; the capability is simply absent, and the
failure surfaces later and further away — as an `AttributeError` reaching for the
RLDS path rather than an honest `ImportError` at install time. That gap between
"installed fine" and "actually works" is what makes this worth documenting.

Installing `array_record` by hand doesn't rescue it. Its wheels are **mistagged**:
the published artifact is `array_record-0.4.1-py310-none-any.whl`, declaring
`Root-Is-Purelib: true` and platform `any`, but the payload contains
`array_record/python/array_record_module.so` — an ELF shared object. Windows pip
will happily download and install that wheel and then fail to load it. So both
routes dead-end: the supported path skips the package, and the manual path installs
a Linux binary onto Windows.

The Linux wheels have none of these problems, so the build shells out rather than
fighting the packaging.

> **One caveat on this rationale.** The `.bat` header also says native pip "can't
> resolve tensorflow at all on newer Python". That half is now stale — as of this
> writing, native Python 3.13 on this machine *does* resolve `tensorflow`, though
> only 2.20.0 and 2.21.0 have `cp313-win_amd64` wheels. The `array_record` marker
> above is the durable reason, and it still holds. Nobody has reproduced the
> original `AttributeError` end-to-end since; the marker is the structural cause,
> but the exact traceback wasn't re-verified.

**One-time setup**, inside a WSL shell:

```sh
curl -LsSf https://astral.sh/uv/install.sh | sh
source ~/.local/bin/env
uv venv --python 3.11 ~/.venvs/rlds
uv pip install --python ~/.venvs/rlds/bin/python -r /mnt/d/black_splat/tools/requirements-rlds.txt
```

Python **3.11** specifically: WSL's default `python3` is newer than anything
tensorflow ships wheels for — the same failure mode as Windows. It's a supported
version, not a magic pin.

**Overrides** (environment variables read by the `.bat`):

| Variable | Default | Meaning |
| --- | --- | --- |
| `WSL_DISTRO` | `Ubuntu` | which distro to run in |
| `RLDS_VENV` | `~/.venvs/rlds` | the venv path inside that distro |

The script translates the repo path itself with `wslpath -a`, so `D:\black_splat`
becomes `/mnt/d/black_splat` without anything hardcoded.

**Where output lands:** `~/tensorflow_datasets/<scene-name>/` **inside WSL's home**,
not under `/mnt/d`. That's deliberate — TFDS looks in `~/tensorflow_datasets` by
default, so anything running inside WSL finds it with no cross-filesystem copy. The
per-scene subfolder exists because the registered TFDS dataset name is a fixed
Python class name, so two scenes sharing an `--output-dir` would collide.

## 2. Getting the dataset back out for native training

[tools/retrain_openvla.bat](../tools/retrain_openvla.bat) runs `finetune.py`
**natively on Windows** — `finetune.py` carries an `if os.name == "nt"` patch that
inits a single-process gloo group, so no torchrun is needed (and nccl has no Windows
build anyway).

That leaves a filesystem gap: the data is in WSL, the trainer is not. The script
bridges it with `robocopy /MIR`:

```
\\wsl$\Ubuntu\home\ben_a\tensorflow_datasets\<scene>   ->   D:\tfds_datasets\<scene>
```

**Why mirror instead of reading `\\wsl$` directly:** training makes many shuffled
passes over the data, and that access pattern across the WSL 9p bridge is slow.
One up-front sequential copy is much cheaper than paying the bridge per batch.

**Two caveats worth knowing:**

- The source path **hardcodes both `Ubuntu` and the username `ben_a`** — unlike the
  rebuild script, it does not honor `WSL_DISTRO`. On a different distro or user
  account, edit `WSL_TFDS_SRC` in the `.bat`.
- **A stopped or wedged distro makes `\\wsl$` paths hang rather than fail fast.** In
  one observed case robocopy sat for 79 minutes before erroring once. When the copy
  fails, the script does *not* abort if a previous mirror exists at
  `D:\tfds_datasets\<scene>\<dataset-name>\` — it prints a `WARNING`, says the copy
  may be stale, and trains against it anyway. That's intentional (it keeps a resume
  alive when the data is already on disk), but **if you rebuilt the dataset and then
  see that warning, stop — you're about to train on the old data.**

## 3. Docker Desktop's backend (policy server)

[policy_server/run_server.bat](../policy_server/run_server.bat) `openvla` runs the
real model in Docker, and on Windows Docker Desktop *is* a WSL2 utility VM. WSL
never appears in the commands, but it shapes three behaviors:

- **GPU passthrough needs WSL2 plus the CUDA-on-WSL driver** — see
  [policy_server/openvla/Dockerfile](../policy_server/openvla/Dockerfile) and the
  `deploy.resources.reservations.devices` block in
  [policy_server/docker-compose.yml](../policy_server/docker-compose.yml). NVIDIA
  only; there's no Mac path and no CPU-only fallback.
- **Cold start is slow** (30s–2min) because the utility VM has to boot before the
  engine answers, which is why `run_server.bat` polls `docker info` instead of doing
  one blind retry.
- **Bind mounts cross the WSL2 filesystem bridge**, which is bad for multi-GB
  sequential reads. That's why `/models` (your `openvla_runs` checkpoints) is treated
  as a *source to copy from once* into the `checkpoints` named volume, rather than
  something `MODEL_ID` points at for routine startups. Same reasoning behind the
  `hf-cache` named volume.

Related: `.docker_data/` at the repo root (gitignored) is the WSL2 VM disk —
images, containers, and downloaded weights — relocated off the C: drive. It is
entirely disposable.

## Troubleshooting

**`\\wsl$` paths hang, or the rebuild script can't find its Python.** The distro is
probably stopped. Check and wake it:

```sh
wsl -l -v              # is Ubuntu "Running" or "Stopped"?
wsl -d Ubuntu -- true  # starts it
```

**"Couldn't find a Python at ~/.venvs/rlds/bin/python".** The one-time setup in
section 1 hasn't been run in that distro, or `RLDS_VENV` points elsewhere.

**Training warns about a stale mirror.** See the second caveat in section 2 — rerun
the rebuild, confirm the distro is up, and re-launch so `robocopy` succeeds.

**A tensorflow import error inside WSL.** Check the venv is Python 3.11
(`~/.venvs/rlds/bin/python -V`); a newer interpreter reproduces the exact failure
that pushed this step off Windows in the first place.
