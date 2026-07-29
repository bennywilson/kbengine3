@echo off
REM Converts every demo in a robomimic *_low_dim.hdf5 file into this engine's
REM trajectory JSON schema, via tools/robomimic_to_trajectory.py --all (see
REM that script's own module doc for the conversion itself -- states/qpos
REM slicing, joint renaming, etc). One-time, offline, native Windows Python
REM (just needs h5py, unlike rebuild_rlds_dataset.bat's WSL+tensorflow path).
REM
REM Usage: convert_robomimic_demos.bat <input.hdf5> <output_dir>
REM   Called by the editor's own "Convert Robomimic Demos..." button (Mujoco
REM   Actor Details panel, Trajectory Playback section) with whatever paths
REM   are typed into the HDF5 Path / Output Dir fields there; pass them by
REM   hand to convert something outside the editor.

setlocal
set INPUT=%~1
set OUTPUT_DIR=%~2

if "%INPUT%"=="" (
    echo Usage: convert_robomimic_demos.bat ^<input.hdf5^> ^<output_dir^>
    echo.
    pause
    exit /b 1
)
if "%OUTPUT_DIR%"=="" (
    echo Usage: convert_robomimic_demos.bat ^<input.hdf5^> ^<output_dir^>
    echo.
    pause
    exit /b 1
)
if not exist "%INPUT%" (
    echo No such file: %INPUT%
    echo.
    pause
    exit /b 1
)

cd /d "%~dp0.."

echo Converting every demo in %INPUT% -^> %OUTPUT_DIR% ...
python tools\robomimic_to_trajectory.py "%INPUT%" --all -o "%OUTPUT_DIR%"
if errorlevel 1 (
    echo.
    echo Conversion failed -- see the error above. Common cause: missing
    echo h5py ^(pip install h5py^) in whatever "python" on PATH resolves to.
    echo.
    pause
    exit /b 1
)

echo.
echo Done. Point a Mujoco Actor's Trajectory field at one of the .json files
echo under %OUTPUT_DIR% ^(Browse…^) to play it back.
echo Press any key to close this window.
pause >nul
