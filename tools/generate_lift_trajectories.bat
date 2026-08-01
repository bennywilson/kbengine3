@echo off
REM Generates scripted Lift demonstrations as trajectory JSON -- see
REM tools\generate_lift_trajectories\src\main.rs's own module docs for what
REM the controller does and why the dataset exists (short version: the
REM converted robomimic demos all approach the cube the same way, so a policy
REM trained on them reacts to "this looks like a Lift attempt" rather than to
REM where the cube actually is; these randomize the approach as well as the
REM object).
REM
REM Usage: generate_lift_trajectories.bat <count> <out-dir> [seed] [start-index]
REM   Called by the editor's own "Generate Scripted Demos…" button. Output is
REM   ordinary TrajectoryClip JSON, byte-compatible with the converted
REM   robomimic clips (same joint layout, same dt), so it feeds Export ->
REM   Rebuild TFDS -> fine-tune completely unchanged.
REM
REM Keep the output in its own directory rather than mixed into the converted
REM clips: the two sets have deliberately different approach distributions and
REM are not interchangeable.
REM
REM Seeded, so the same seed regenerates a dataset exactly -- which is why the
REM output is gitignored and the seed is the thing worth recording.
REM
REM Builds in release first (a debug build runs the physics far too slowly to
REM be practical); the build is a no-op once warm.

setlocal
set COUNT=%1
set OUT_DIR=%~2
set SEED=%3
set START_INDEX=%4
if "%COUNT%"=="" set COUNT=200
if "%SEED%"=="" set SEED=1
REM Exports are namespaced by each clip's own filename, so two clip folders
REM that both start at demo_0 overwrite each other's exports. Offset one set
REM past the other to export both into a single training set.
if "%START_INDEX%"=="" set START_INDEX=0
if "%OUT_DIR%"=="" (
    echo Usage: generate_lift_trajectories.bat ^<count^> ^<out-dir^> [seed] [start-index]
    echo.
    pause
    exit /b 1
)

cd /d "%~dp0generate_lift_trajectories"
echo Building the generator ^(release^)...
cargo build --release
if errorlevel 1 (
    echo.
    echo Build failed -- see the error above.
    echo.
    pause
    exit /b 1
)

echo.
echo Generating %COUNT% demo^(s^) into %OUT_DIR% ^(seed %SEED%, from demo_%START_INDEX%^)...
echo.
target\release\black_splat_generate_lift_trajectories.exe ^
    --count %COUNT% ^
    --out-dir "%OUT_DIR%" ^
    --seed %SEED% ^
    --start-index %START_INDEX%

echo.
echo Done (or see error above). Bind one of the generated clips in the
echo Trajectory field, then Export Training Data as usual.
echo Press any key to close this window.
pause >nul
