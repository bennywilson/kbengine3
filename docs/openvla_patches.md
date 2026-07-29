# The OpenVLA clone's local patches

The fine-tuner is not in this repo. It lives in a separate clone of
[openvla/openvla](https://github.com/openvla/openvla) at `D:\openvla`, which
[tools/retrain_openvla.bat](../tools/retrain_openvla.bat) shells into via
`OPENVLA_REPO`. That clone is **not** a submodule and not vendored — it is a
sibling checkout with seven local changes across three files, without which
training here does not run at all.

A copy of the exact diff is checked in at
[tools/openvla_black_splat.patch](../tools/openvla_black_splat.patch) so the
changes are versioned alongside the project that depends on them, and survive
the clone being deleted or re-cloned.

## The clone at a glance

| | |
| --- | --- |
| Path | `D:\openvla` (override with `OPENVLA_REPO`) |
| `origin` | `github.com/bennywilson/openvla` (fork) |
| `upstream` | `github.com/openvla/openvla` |
| Branch | `black-splat` |
| Base commit | `c8f03f4` — upstream `main`, 2025-03-23 |
| Patch commit | `1b90161` — 3 files, +148 / −52 |
| Training venv | `C:\openvla_venv` (Python 3.10, see [Environment pins](#environment-pins-not-in-the-patch)) |

> **Be careful with `git` in that clone.** A bare `git checkout main`, `pull`, or
> `stash` silently reverts every change below, and the next fine-tune dies with a
> bare `KeyError` *after* the 7B model has finished loading. Recovery is
> `git checkout black-splat`, or re-applying the patch (see
> [Reapplying to a fresh clone](#reapplying-to-a-fresh-clone)).

## What's patched, and why

| # | File | Change |
| --- | --- | --- |
| 1 | `prismatic/vla/datasets/rlds/oxe/configs.py` | Register each scene's dataset shape |
| 2 | `prismatic/vla/datasets/rlds/oxe/transforms.py` | Register a no-op standardization transform |
| 3 | `vla-scripts/finetune.py` | Init a single-process gloo group on Windows |
| 4 | `vla-scripts/finetune.py` | Skip DDP entirely when `num_processes == 1` |
| 5 | `vla-scripts/finetune.py` | Adapter-only periodic checkpoints; merge once at the end |
| 6 | `vla-scripts/finetune.py` | Gate the checkpoint save on the accumulation boundary |
| 7 | `vla-scripts/finetune.py` | `--resume_adapter_dir`, and metrics echoed to stdout |

### 1–2. Dataset registration

**Why:** OpenVLA has no dataset auto-discovery. Every dataset name must be
pre-registered in *both* `OXE_DATASET_CONFIGS` (observation/action shape) and
`OXE_STANDARDIZATION_TRANSFORMS` (a per-dataset transform function), or
`materialize.py` raises a bare `KeyError` — and it does so only after the 7B
model has loaded, so each miss costs about a minute of startup to discover.

Both entries mirror `roboturk`, the closest existing shape match: one primary
camera, no wrist or secondary view, no proprioceptive state
(`StateEncoding.NONE`), EEF delta-pose plus gripper (`ActionEncoding.EEF_POS`).

The dataset name is **per scene** — `build_rlds_dataset.py`'s
`_builder_class_for_scene` produces `black_splat_<scene>`, so
`black_splat_tool_hang` and `black_splat_panda_lift` are both registered today.
Every scene shares the identical shape, so **adding a scene is two lines**: one
entry in `configs.py` and one line in `transforms.py`'s registry, both pointing
at the same shared `black_splat_dataset_transform`.

That transform is a deliberate **no-op**: `build_rlds_dataset.py` already emits
`action` and `language_instruction` in the flat top-level shape this pipeline
wants. It is also the one place where this data intentionally disagrees with the
rest of OXE — every other dataset's transform normalizes the gripper to
`+1 = open, 0 = closed`, and this one stays in its native `0 = open, 1 = closed`
convention because it is always fine-tuned and served solo, and
`MujocoScene::apply_policy_action` (`src/mujoco.rs`) already expects it that way.
Folding this dataset into a mixture with other OXE data would require flipping
it; the file says so at the call site.

### 3. Init the distributed process group on Windows

**Why:** `torchrun --standalone` fails outright on native Windows
(`RendezvousConnectionError` from its C10d rendezvous backend), and running
`finetune.py` directly hits two more walls: `prismatic`'s `overwatch` module
calls `accelerate.PartialState()` as an *import-time* side effect — defaulting to
`nccl`, which has no Windows build — and `DistributedDataParallel` needs *some*
initialized group even for one process.

The patch initializes a single-process `gloo` group at the very top of the file,
before the `prismatic` imports, using a `file://` init method. `file://`
specifically, because `env://` and `tcp://` init both hit flaky TCPStore hostname
resolution on native Windows. Every later `PartialState()` sees
`torch.distributed.is_initialized() == True` and skips its own attempt, so no
call site needs patching.

### 4. Don't wrap in DDP on a single process

**Why:** even with the group initialized, DDP's reentrant-backward gradient
bucketing (required by `find_unused_parameters=True`) conflicts with this model's
gradient checkpointing — `RuntimeError: Expected to mark a variable ready only
once`. That is a real interaction bug, not a tunable flag, and with one GPU there
is no gradient to sync anyway.

The wrap is now conditional on `distributed_state.num_processes > 1`, with
`vla_module` tracking the unwrapped model for the handful of `.module` accesses
(config, vision backbone, `save_pretrained`) that never needed DDP.

### 5. Adapter-only checkpoints, one merge after training

**Why:** upstream merges LoRA into a full model on *every* checkpoint, which
loads a second full-precision bf16 7B copy on CPU while the quantized training
model and TF dataloader are still resident. On this 32GB machine that segfaulted
a real run (`lora_run1`) ~53 minutes in, right after saving step 2000. VRAM was
never the problem — it was the CPU-side spike.

Upstream's own comment calls the merge deferrable ("can be done post-hoc to speed
up training"), so periodic checkpoints now save only the small LoRA adapter, and
the single merge happens after the training loop exits — with the training model
and optimizer `del`'d, `gc.collect()`'d, and `torch.cuda.empty_cache()`'d first
to maximize headroom.

### 6. Gate the save on the accumulation boundary

**Why:** the save block sat outside the `(batch_idx + 1) % grad_accumulation_steps
== 0` guard, but `gradient_step_idx` is constant across all micro-batches in an
accumulation window. So it fired **once per micro-batch** — 8 redundant saves per
checkpoint at the current settings — and could fire *before* that step's
`optimizer.step()`, writing the previous step's weights.

This is a genuine upstream bug rather than a Windows workaround, and is the one
change here that would be worth a PR.

### 7. `--resume_adapter_dir` and stdout metrics

**`--resume_adapter_dir`** loads an existing adapter via
`PeftModel.from_pretrained(vla, dir, is_trainable=True)` instead of a fresh
random LoRA init, so a crashed run continues from its last saved adapter.
`is_trainable=True` matters — `from_pretrained` otherwise loads adapters frozen
for inference. Note this restores adapter *weights* only: optimizer momentum and
the step counter are not saved, which is acceptable at this scale with no LR
schedule. [tools/retrain_openvla.bat](../tools/retrain_openvla.bat) exposes it as
an optional second argument and refuses a path with no `adapter_config.json`
rather than silently starting over.

**Stdout metrics** print `loss` / `action_accuracy` / `l1_loss` every 10 gradient
steps, alongside the existing W&B logging. Training here runs with
`WANDB_MODE=disabled`, and the very first fine-tune's mode collapse (constant
action, image ignored) went unnoticed until a post-hoc eval precisely because
nothing recorded whether loss fell or accuracy rose.

## Reapplying to a fresh clone

```sh
git clone https://github.com/openvla/openvla D:/openvla
git -C D:/openvla checkout -b black-splat c8f03f4
git -C D:/openvla apply D:/black_splat/tools/openvla_black_splat.patch
```

The patch is generated against `c8f03f4` exactly. Against a newer upstream, try
`git apply -3` so conflicts surface as merge markers instead of a flat refusal.

**If you edit anything in the clone, regenerate the patch** — nothing does this
automatically:

```sh
git -C D:/openvla diff c8f03f4 black-splat > D:/black_splat/tools/openvla_black_splat.patch
```

## Upstreamability

Only change 6 is a real upstream bug fix. Changes 3–4 are Windows-only
workarounds, 1–2 are project-specific registrations, and 5 trades a documented
convenience for low-RAM survival. None of the rest belongs upstream as written.

## Environment pins (not in the patch)

The training venv is `C:\openvla_venv` (Python **3.10** — 3.13 has no
`torch==2.2.0` wheel), with `HF_HOME=D:\hf_cache`. These pins are not in the
clone's `pyproject.toml` and took real trial-and-error to find; none is
guessable from the README, which assumes Linux:

| Pin | Why |
| --- | --- |
| `numpy<2` | torch 2.2.0 predates NumPy 2's C-API |
| `bitsandbytes==0.43.1` | latest declares `torch>=2.4` and silently upgrades torch, breaking the whole chain |
| `accelerate==0.30.1` | newer `dispatch_model` calls `.to(device)` on a 4-bit model, which transformers 4.40.1 rejects outright |
| `tensorflow-metadata==1.17.3` | 1.21's protobuf stubs need `protobuf>=5.26`, conflicting with `tensorflow==2.15`'s `protobuf<5` |
| `protobuf==4.25.9` | pin explicitly, or the resolver grabs something far too new for TF |
| `tensorflow_datasets==4.9.10` | 4.9.3 unconditionally imports `resource`, a POSIX-only stdlib module |

Quantization is **required, not optional**: the GPU is a 16GB *laptop* RTX 4090,
so `--use_quantization True` (4-bit NF4) is what makes even `batch_size=1` fit.

## Known gaps

Deliberately not done, in rough risk order:

- **No training log.** `retrain_openvla.bat` has no redirect, so the loss curve
  lives only in console scrollback and has been reconstructed by hand from pasted
  text more than once.
- **Sleep prevention is global, not targeted.** A drive dropout killed a 5-hour
  run mid-checkpoint; the fix applied was `powercfg /change disk-timeout-ac 0` and
  `standby-timeout-ac 0` **machine-wide**. Windows measures sleep idle by *user
  input*, not GPU load, so an unattended run trips the timer with the GPU pinned.
  The clean fix is a `ctypes` `SetThreadExecutionState` power request inside
  `finetune.py` — i.e. an eighth patch — after which the global setting can be
  restored (`powercfg /change standby-timeout-ac 180`).

Related: [docs/wsl.md](wsl.md) for where each pipeline step actually runs, and
[policy_server/README.md](../policy_server/README.md) for serving the resulting
checkpoints.
