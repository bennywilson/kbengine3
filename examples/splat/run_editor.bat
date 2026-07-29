@echo off
REM Runs the native editor (this crate, black_splat_splat_demo) in release
REM mode -- matches how it's actually been run day-to-day (see the crash
REM traces under target\release\), not `cargo run`'s debug default, which is
REM meaningfully slower for a GPU-bound app like this one.
REM
REM Output is teed to logs\editor_<timestamp>.log alongside the live console
REM (cmd.exe has no built-in `tee`, hence the one-line PowerShell relay --
REM same pattern tools\retrain_openvla.bat uses for train.log). Without this,
REM a crash's only record is whatever's still in the console's own
REM scrollback, and copying text out of a Windows console is its own fight --
REM exactly what made one real wgpu crash here hard to hand off.

setlocal enabledelayedexpansion
cd /d "%~dp0"
if not exist logs mkdir logs
for /f %%i in ('powershell -NoProfile -Command "Get-Date -Format yyyyMMdd_HHmmss"') do set TS=%%i
set EDITOR_LOG=logs\editor_%TS%.log

echo Starting the editor (release build)...
echo Full console output also going to: %EDITOR_LOG%
echo.
cargo run --release 2>&1 | powershell -NoProfile -Command "$input | Tee-Object -FilePath '%EDITOR_LOG%'"

echo.
echo Exited (or see error above). Full log: %EDITOR_LOG%
pause
