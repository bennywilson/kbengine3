@echo off
REM Runs tools/eval_openvla.py against one scene's held-out demos -- see
REM that script's own module doc for what it measures and why. Needs a
REM TFDS rebuild done with --holdout set (the editor's "Rebuild TFDS
REM Dataset..." button has a holdout field); without that, there's no
REM unseen data to evaluate against and this aborts.
REM
REM Usage: eval_openvla.bat <scene-name> <adapter-dir>
REM   Called by the editor's own "Run Eval..." button with the current
REM   scene's name and whatever's in the Eval adapter dir field.
REM
REM Runs natively on Windows against the same C:\openvla_venv training venv
REM retrain_openvla.bat uses (torch+transformers+peft) -- no tensorflow
REM needed here, this reads the raw manifest.jsonl exports directly, not
REM the TFDS/RLDS output. Override OPENVLA_VENV/HF_HOME if yours differ.

setlocal
set SCENE_NAME=%1
if "%SCENE_NAME%"=="" set SCENE_NAME=unsaved
set ADAPTER_DIR=%~2
if "%ADAPTER_DIR%"=="" (
    echo Usage: eval_openvla.bat ^<scene-name^> ^<adapter-dir^>
    echo.
    pause
    exit /b 1
)
if not exist "%ADAPTER_DIR%\adapter_config.json" (
    echo %ADAPTER_DIR% doesn't look like an adapter checkpoint ^(no adapter_config.json found there^).
    echo.
    pause
    exit /b 1
)

if not defined OPENVLA_VENV set OPENVLA_VENV=C:\openvla_venv
if not defined HF_HOME set HF_HOME=D:\hf_cache

if not exist "%OPENVLA_VENV%\Scripts\python.exe" (
    echo Couldn't find %OPENVLA_VENV%\Scripts\python.exe -- set OPENVLA_VENV
    echo to point at your training venv ^(torch+transformers+peft^).
    echo.
    pause
    exit /b 1
)

cd /d "%~dp0.."
echo Evaluating %ADAPTER_DIR% against %SCENE_NAME%'s held-out demos...
echo.
"%OPENVLA_VENV%\Scripts\python.exe" tools\eval_openvla.py ^
    --adapter-dir "%ADAPTER_DIR%" ^
    --data-dir "examples\splat\resources\openvla_datasets\%SCENE_NAME%"

echo.
echo Done (or see error above). Press any key to close this window.
pause >nul
