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
REM (100,000 -> 2,000): the dataclass default is sized for OpenVLA's
REM original OXE-scale training sets (hundreds of thousands of frames), and
REM tf.data insists on filling that whole buffer before training starts --
REM against this dataset's few hundred/thousand frames, that's a long
REM "filling shuffle buffer" stall for no shuffling benefit (a real one
REM cost ~1.5 hours doing nothing else here).
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

REM finetune.py runs natively on Windows, but the TFDS dataset was built
REM inside WSL (see rebuild_rlds_dataset.bat -- native Windows has no working
REM tensorflow-datasets install on this machine). Mirror it to a native path
REM first so training's many shuffled passes over the data don't all cross
REM the WSL 9p bridge, which is slow for this kind of access pattern.
set WSL_TFDS_SRC=\\wsl$\Ubuntu\home\ben_a\tensorflow_datasets\%SCENE_NAME%
set NATIVE_TFDS_DIR=D:\tfds_datasets\%SCENE_NAME%
echo Mirroring %WSL_TFDS_SRC% to %NATIVE_TFDS_DIR% ...
robocopy "%WSL_TFDS_SRC%" "%NATIVE_TFDS_DIR%" /MIR /NFL /NDL /NJH /NJS >nul
if %errorlevel% GEQ 8 (
    echo robocopy failed -- has %SCENE_NAME% actually been built yet?
    echo Run "Rebuild TFDS Dataset" first.
    echo.
    pause
    exit /b 1
)

REM Read back the exact TFDS dataset name build_rlds_dataset.py registered
REM for this scene (see that script's _builder_class_for_scene) rather than
REM guessing it here -- a name computed independently in batch script could
REM drift from whatever TFDS's class-name-to-dataset-name rule actually
REM produced, and a mismatch means finetune.py can't find the data at all.
set DATASET_NAME_FILE=%~dp0..\examples\splat\resources\openvla_datasets\%SCENE_NAME%\dataset_name.txt
if not exist "%DATASET_NAME_FILE%" (
    echo Couldn't find %DATASET_NAME_FILE%.
    echo Run "Rebuild TFDS Dataset" first ^(or rebuild_rlds_dataset.bat %SCENE_NAME%^).
    echo.
    pause
    exit /b 1
)
set /p DATASET_NAME=<"%DATASET_NAME_FILE%"

for /f %%i in ('powershell -NoProfile -Command "Get-Date -Format yyyyMMdd_HHmmss"') do set TS=%%i
set RUN_NAME=retrain_%SCENE_NAME%_%TS%
set RUN_DIR=D:\openvla_runs\%RUN_NAME%

set RESUME_ARG=
if not "%RESUME_ADAPTER_DIR%"=="" set RESUME_ARG=--resume_adapter_dir "%RESUME_ADAPTER_DIR%"

echo.
echo Starting fine-tune -- run: %RUN_NAME%
echo Logs/checkpoints: %RUN_DIR%
if not "%RESUME_ADAPTER_DIR%"=="" echo Resuming LoRA weights from: %RESUME_ADAPTER_DIR%
echo.
cd /d "%OPENVLA_REPO%"
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
    --shuffle_buffer_size 2000 ^
    %RESUME_ARG%

echo.
echo Training stopped (or see error above). Checkpoints under %RUN_DIR%.
echo Ctrl+C anytime to stop early -- save_latest_checkpoint_only keeps just
echo the most recent save, so the last checkpoint before stopping is what
echo you get (matches how lora_run2 was stopped at step 500 to evaluate).
echo Press any key to close this window.
pause >nul
