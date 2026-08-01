@echo off
REM Kicks off a LoRA fine-tune of OpenVLA-7b on one scene's TFDS dataset (see
REM rebuild_rlds_dataset.bat -- run that first; this doesn't rebuild it).
REM
REM Runs natively on Windows, not WSL/Docker: vla-scripts/finetune.py has its
REM own Windows patch (see that file's `if os.name == "nt"` block) that inits
REM a single-process gloo "distributed" group up front, so plain
REM `python finetune.py` works standalone -- no torchrun needed for a single
REM GPU, and nccl (torchrun's usual backend) has no Windows build anyway.
REM
REM Usage: retrain_openvla.bat <scene-name> [resume-adapter-dir]
REM   Called with the current scene's name (and, if set, the Resume adapter
REM   dir field) by the editor's own "Fine-tune OpenVLA..." button. Defaults
REM   to "unsaved" for manual runs, matching the editor's own fallback
REM   naming. resume-adapter-dir, if given, continues LoRA weights from an
REM   existing adapter checkpoint (e.g.
REM   D:\openvla_runs\lora_run2\adapter-tmp\openvla-7b+...+q-4bit) instead
REM   of starting from scratch -- doesn't restore optimizer state or the
REM   step counter, just the learned weights (see finetune.py's own
REM   --resume_adapter_dir handling).
REM
REM Hyperparameters below match the last known-good run (lora_run2 -- see
REM D:\openvla_runs\lora_run2\train.log): batch size 8, LoRA rank 32, lr
REM 5e-4, 4-bit quantized (a single 24GB GPU can't fit unquantized bf16-7B +
REM LoRA training at a useful batch size -- see openvla's own README on the
REM ~72GB unquantized batch-16 requirement), image_aug off (this dataset is
REM small enough that augmentation wasn't worth the added training-time
REM variance). Edit the constants below directly for a different config --
REM this intentionally doesn't expose every finetune.py flag as its own
REM override, since that's most of its CLI surface.
REM
REM shuffle_buffer_size is the one default explicitly overridden below
REM (100,000 -> 20,000): the dataclass default is sized for OpenVLA's
REM original OXE-scale training sets (hundreds of thousands of frames), and
REM tf.data insists on filling that whole buffer before training starts --
REM against a dataset this size that's a long "filling shuffle buffer" stall
REM for no shuffling benefit (a real one cost ~1.5 hours doing nothing else
REM here).
REM
REM It must still comfortably EXCEED the dataset, though, or the "shuffle"
REM degenerates into a sliding window over whatever order
REM build_rlds_dataset.py emitted episodes in -- which is lexicographic by
REM folder name (demo_0, demo_1, demo_10, demo_100, ...), i.e. not random at
REM all. This bit once: 2,000 covered 94% of a 2,130-sample dataset and was
REM effectively a full shuffle, but after the dataset grew to 7,691 samples
REM the same 2,000 covered only 26%. Training then fit each slice as the
REM window slid over it and forgot the last -- action_accuracy climbed to
REM 0.82 by step 700 and collapsed to ~0.37 by step 1800, while reporting
REM "how well does it fit the current window" rather than anything about the
REM dataset. 20,000 leaves room for the set to grow several times over
REM before this needs revisiting; raise it if the training set ever
REM approaches that.
REM
REM WANDB_MODE=disabled: no W&B account needed. finetune.py separately
REM echoes loss/action_accuracy to stdout every 10 steps (see its own
REM comment on why -- the first run here was WANDB_MODE=disabled with no
REM stdout echo, and its silent mode collapse went unnoticed until a
REM post-hoc eval caught it), so this .bat's own console output is still a
REM readable training curve.
REM
REM Override OPENVLA_VENV/OPENVLA_REPO/HF_HOME if yours differ.

setlocal enabledelayedexpansion
set SCENE_NAME=%1
if "%SCENE_NAME%"=="" set SCENE_NAME=unsaved
set RESUME_ADAPTER_DIR=%~2
if not "%RESUME_ADAPTER_DIR%"=="" (
    if not exist "%RESUME_ADAPTER_DIR%\adapter_config.json" (
        echo %RESUME_ADAPTER_DIR% doesn't look like an adapter checkpoint
        echo ^(no adapter_config.json found there^). Aborting rather than
        echo silently starting from scratch.
        echo.
        pause
        exit /b 1
    )
)

if not defined OPENVLA_VENV set OPENVLA_VENV=C:\openvla_venv
if not defined OPENVLA_REPO set OPENVLA_REPO=D:\openvla
if not defined HF_HOME set HF_HOME=D:\hf_cache
set WANDB_MODE=disabled

if not exist "%OPENVLA_VENV%\Scripts\python.exe" (
    echo Couldn't find %OPENVLA_VENV%\Scripts\python.exe -- set OPENVLA_VENV
    echo to point at your training venv ^(torch+transformers+peft, see
    echo pyproject.toml in %OPENVLA_REPO%^).
    echo.
    pause
    exit /b 1
)

REM Read back the exact TFDS dataset name build_rlds_dataset.py registered
REM for this scene (see that script's _builder_class_for_scene) rather than
REM guessing it here -- a name computed independently in batch script could
REM drift from whatever TFDS's class-name-to-dataset-name rule actually
REM produced, and a mismatch means finetune.py can't find the data at all.
REM Read before the mirror below, which needs it to identify a usable
REM fallback copy.
set DATASET_NAME_FILE=%~dp0..\examples\splat\resources\openvla_datasets\%SCENE_NAME%\dataset_name.txt
if not exist "%DATASET_NAME_FILE%" (
    echo Couldn't find %DATASET_NAME_FILE%.
    echo Run "Rebuild TFDS Dataset" first ^(or rebuild_rlds_dataset.bat %SCENE_NAME%^).
    echo.
    pause
    exit /b 1
)
set /p DATASET_NAME=<"%DATASET_NAME_FILE%"

REM finetune.py runs natively on Windows, but the TFDS dataset was built
REM inside WSL (see rebuild_rlds_dataset.bat -- native Windows has no working
REM tensorflow-datasets install on this machine). Mirror it to a native path
REM first so training's many shuffled passes over the data don't all cross
REM the WSL 9p bridge, which is slow for this kind of access pattern.
set WSL_TFDS_SRC=\\wsl$\Ubuntu\home\ben_a\tensorflow_datasets\%SCENE_NAME%
set NATIVE_TFDS_DIR=D:\tfds_datasets\%SCENE_NAME%
echo Mirroring %WSL_TFDS_SRC% to %NATIVE_TFDS_DIR% ...
robocopy "%WSL_TFDS_SRC%" "%NATIVE_TFDS_DIR%" /MIR /NFL /NDL /NJH /NJS >nul
if %errorlevel% LSS 8 goto tfds_ready

REM An unreadable WSL source is only fatal when no usable native mirror
REM already exists. A stopped or wedged Ubuntu distro makes \\wsl$ paths hang
REM instead of failing fast -- robocopy sat doing nothing for 79 minutes
REM before erroring once -- and aborting there strands a resume whose data is
REM already on disk from an earlier successful mirror. Warn and continue
REM instead; the only real hazard is a mirror predating a newer rebuild,
REM which the warning calls out explicitly.
if not exist "%NATIVE_TFDS_DIR%\%DATASET_NAME%\" (
    echo robocopy couldn't read %WSL_TFDS_SRC%, and there is no existing
    echo mirror at %NATIVE_TFDS_DIR%\%DATASET_NAME%\ to fall back on.
    echo Has %SCENE_NAME% been built yet? Run "Rebuild TFDS Dataset" first.
    echo.
    pause
    exit /b 1
)
echo.
echo WARNING: couldn't reach %WSL_TFDS_SRC% ^(is the Ubuntu WSL distro running?^)
echo Training against the EXISTING mirror at %NATIVE_TFDS_DIR%.
echo That copy is STALE if the dataset has been rebuilt since it was made.
echo.
:tfds_ready

for /f %%i in ('powershell -NoProfile -Command "Get-Date -Format yyyyMMdd_HHmmss"') do set TS=%%i
set RUN_NAME=retrain_%SCENE_NAME%_%TS%
set RUN_DIR=D:\openvla_runs\%RUN_NAME%

set RESUME_ARG=
if not "%RESUME_ADAPTER_DIR%"=="" set RESUME_ARG=--resume_adapter_dir "%RESUME_ADAPTER_DIR%"

if not exist "%RUN_DIR%" mkdir "%RUN_DIR%"
set TRAIN_LOG=%RUN_DIR%\train.log

echo.
echo Starting fine-tune -- run: %RUN_NAME%
echo Logs/checkpoints: %RUN_DIR%
echo Full console output also going to: %TRAIN_LOG%
if not "%RESUME_ADAPTER_DIR%"=="" echo Resuming LoRA weights from: %RESUME_ADAPTER_DIR%
echo.
cd /d "%OPENVLA_REPO%"
REM cmd.exe has no built-in `tee` -- piping through a one-line PowerShell
REM Tee-Object keeps this window's live output (so save_steps/action_accuracy
REM are still watchable in real time, see this file's own WANDB_MODE comment
REM above) while also writing a durable copy. Without this, a crash mid-run
REM only leaves whatever's still in the console's own scrollback -- exactly
REM what made a real wgpu crash's log hard to recover from once already.
"%OPENVLA_VENV%\Scripts\python.exe" vla-scripts\finetune.py ^
    --vla_path "openvla/openvla-7b" ^
    --data_root_dir "%NATIVE_TFDS_DIR%" ^
    --dataset_name %DATASET_NAME% ^
    --run_root_dir "%RUN_DIR%" ^
    --adapter_tmp_dir "%RUN_DIR%\adapter-tmp" ^
    --lora_rank 32 ^
    --lora_dropout 0.0 ^
    --batch_size 1 ^
    --grad_accumulation_steps 8 ^
    --learning_rate 5e-4 ^
    --use_quantization True ^
    --image_aug False ^
    --save_steps 100 ^
    --shuffle_buffer_size 20000 ^
    %RESUME_ARG% 2>&1 | powershell -NoProfile -Command "$input | Tee-Object -FilePath '%TRAIN_LOG%'"

echo.
echo Training stopped (or see error above). Checkpoints under %RUN_DIR%.
echo Full log: %TRAIN_LOG%
echo Ctrl+C anytime to stop early -- save_latest_checkpoint_only keeps just
echo the most recent save, so the last checkpoint before stopping is what
echo you get (matches how lora_run2 was stopped at step 500 to evaluate).
echo Press any key to close this window.
pause >nul
