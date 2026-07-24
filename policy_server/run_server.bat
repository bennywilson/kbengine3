@echo off
REM Spin up a policy server on port 8000 for the splat demo's Policy Control.
REM The demo's "Server URL" defaults to http://localhost:8000, so either mode
REM below is reachable with no further config.
REM
REM Usage:
REM   run_server.bat stub      Canned-response server (no GPU / no Docker).
REM                            Returns a fixed action for ANY instruction --
REM                            it has no model, so it can't follow prompts. Use
REM                            it to test the capture -> HTTP -> apply -> sim
REM                            pipeline itself, independent of model quality.
REM   run_server.bat openvla   Real OpenVLA-7b in Docker (needs an NVIDIA GPU +
REM                            WSL2 CUDA driver). First run downloads weights.
REM   run_server.bat           Defaults to stub.
REM
REM Before starting, this frees port 8000 from a leftover *python* server (a
REM previous stub, or an old `http.server`) so you don't hit a bind error. It
REM deliberately does NOT kill a Docker-held port -- Docker manages that itself
REM (the openvla path recreates its container), and force-killing Docker's port
REM proxy is a good way to wedge Docker Desktop.

setlocal
set MODE=%1
if "%MODE%"=="" set MODE=stub

echo Freeing port 8000 from any leftover python server...
powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 8000 -State Listen -ErrorAction SilentlyContinue | ForEach-Object { $p = Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue; if ($p -and $p.ProcessName -like 'python*') { Stop-Process -Id $p.Id -Force } }"

cd /d "%~dp0"

if /I "%MODE%"=="stub" (
    echo Starting stub policy server on http://localhost:8000 ...
    python stub_server.py
) else if /I "%MODE%"=="openvla" (
    echo Starting OpenVLA policy server in Docker on http://localhost:8000 ...
    echo First run downloads the model weights -- wait for "Uvicorn running" before enabling Policy Control.
    docker compose --profile openvla up --build
) else (
    echo Unknown mode "%MODE%". Usage: run_server.bat [stub^|openvla]
    exit /b 1
)
