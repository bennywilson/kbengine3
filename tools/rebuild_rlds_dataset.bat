@echo off
REM Rebuilds the OpenVLA/RLDS TFDS dataset from one scene's exports under
REM examples\splat\resources\openvla_datasets\<scene-name>\ (see
REM build_rlds_dataset.py's own module doc, and policy_dataset.rs's
REM export_dir_for for why exports are namespaced per scene). The engine
REM never touches TFDS itself, so re-exporting clips in the editor (Training
REM Data Export -> "Re-export Existing...") does nothing to a training run
REM until this has been re-run too.
REM
REM Usage: rebuild_rlds_dataset.bat <scene-name> [holdout-demos]
REM   Called with the current scene's name (and whatever's in the Holdout
REM   demos field) by the editor's own "Rebuild TFDS Dataset..." button;
REM   pass args explicitly to rebuild a different scene's exports by hand.
REM   Defaults to "unsaved" (matches the editor's own fallback before a
REM   scene has ever been saved/loaded this session). holdout-demos, if
REM   given, is a comma-separated list (e.g. demo_3,demo_17) excluded from
REM   the build and reserved for tools/eval_openvla.py -- see
REM   build_rlds_dataset.py's own --holdout doc for why.
REM
REM Runs inside WSL, not native Windows Python: as of this writing there is
REM no Windows wheel combination of tensorflow + tensorflow-datasets that
REM both installs cleanly AND actually works (native pip installs either
REM fail to resolve tensorflow at all on newer Python, or install a
REM tensorflow-datasets whose `rlds` submodule can't finish importing against
REM whatever tensorflow build did land -- see the AttributeError this used
REM to throw). Linux wheels for both are mature, so this shells out to WSL
REM and a small `uv`-managed venv there instead of fighting Windows pins
REM again. One-time setup inside WSL (Ubuntu by default -- override
REM WSL_DISTRO if yours differs):
REM   curl -LsSf https://astral.sh/uv/install.sh | sh
REM   source ~/.local/bin/env
REM   uv venv --python 3.11 ~/.venvs/rlds
REM   uv pip install --python ~/.venvs/rlds/bin/python -r /mnt/d/black_splat/tools/requirements-rlds.txt
REM (Python 3.11 specifically: WSL's own default python3 is newer than
REM anything tensorflow currently ships wheels for, same failure mode as
REM Windows -- 3.11 is a safely-supported version, not a magic pin.)
REM Override RLDS_VENV if yours lives somewhere other than ~/.venvs/rlds.

setlocal
cd /d "%~dp0.."

set SCENE_NAME=%1
if "%SCENE_NAME%"=="" set SCENE_NAME=unsaved
set HOLDOUT=%~2

if not defined WSL_DISTRO set WSL_DISTRO=Ubuntu
if not defined RLDS_VENV set RLDS_VENV=~/.venvs/rlds

wsl -d %WSL_DISTRO% -- bash -lc "test -x %RLDS_VENV%/bin/python"
if errorlevel 1 (
    echo Couldn't find a Python at %RLDS_VENV%/bin/python inside the %WSL_DISTRO% WSL distro.
    echo One-time setup ^(inside a WSL shell^):
    echo   curl -LsSf https://astral.sh/uv/install.sh ^| sh ^&^& source ~/.local/bin/env
    echo   uv venv --python 3.11 %RLDS_VENV%
    echo   uv pip install --python %RLDS_VENV%/bin/python -r /mnt/d/black_splat/tools/requirements-rlds.txt
    echo.
    echo Done ^(or see error above^). Press any key to close this window.
    pause >nul
    exit /b 1
)

for /f "delims=" %%i in ('wsl -d %WSL_DISTRO% wslpath -a "%CD%"') do set WSL_REPO_ROOT=%%i

REM --output-dir is also namespaced per scene: the TFDS builder's registered
REM dataset name (BlackSplatToolHang / "black_splat_tool_hang") is a fixed
REM Python class name, not scene-aware, so two scenes writing to the exact
REM same --output-dir would still land in the same
REM <output-dir>/black_splat_tool_hang/<version>/ and could overwrite or
REM version-collide with each other. This keeps different scenes' TFDS
REM output on disjoint paths; point finetune.py's --data_root_dir at this
REM same per-scene folder. Written under WSL's own home (not /mnt/d) so
REM finetune.py -- wherever it ends up running -- picks it up via TFDS's
REM default `~/tensorflow_datasets` lookup without a slow cross-filesystem
REM copy; if finetune.py runs natively on Windows instead, copy this folder
REM over by hand or point --data_root_dir at its \\wsl$\%WSL_DISTRO%\home\... UNC path.
echo Rebuilding TFDS dataset from examples\splat\resources\openvla_datasets\%SCENE_NAME% ...
if not "%HOLDOUT%"=="" echo Holding out: %HOLDOUT%
wsl -d %WSL_DISTRO% -- bash -lc "cd '%WSL_REPO_ROOT%' && %RLDS_VENV%/bin/python tools/build_rlds_dataset.py --data-dir examples/splat/resources/openvla_datasets/%SCENE_NAME% --output-dir ~/tensorflow_datasets/%SCENE_NAME% --holdout '%HOLDOUT%'"

echo.
echo Done (or see error above). Press any key to close this window.
pause >nul
