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

REM enabledelayedexpansion: the Docker-wait retry loop below sets and rereads
REM WAITED/DOCKER_DESKTOP_EXE inside the same ( ) block. Plain %VAR% inside a
REM block substitutes once, using the value from before the block started --
REM !VAR! (delayed expansion) re-reads the live value on each line instead.
setlocal enabledelayedexpansion
set MODE=%1
if "%MODE%"=="" set MODE=stub

echo Freeing port 8000 from any leftover python server...
powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 8000 -State Listen -ErrorAction SilentlyContinue | ForEach-Object { $p = Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue; if ($p -and $p.ProcessName -like 'python*') { Stop-Process -Id $p.Id -Force } }"

cd /d "%~dp0"

if /I "%MODE%"=="stub" (
    echo Starting stub policy server on http://localhost:8000 ...
    python stub_server.py
) else if /I "%MODE%"=="openvla" (
    where /q docker
    if errorlevel 1 (
        echo Docker isn't on PATH -- install Docker Desktop:
        echo   https://www.docker.com/products/docker-desktop/
        echo.
        pause
        exit /b 1
    )

    docker info >nul 2>&1
    if errorlevel 1 (
        echo Docker daemon isn't responding -- starting Docker Desktop...
        REM Docker Desktop installs either machine-wide under ProgramFiles or
        REM per-user under LOCALAPPDATA, so probe both rather than assuming --
        REM this machine has the per-user layout, and the ProgramFiles-only
        REM default reported "couldn't find Docker Desktop" while Docker was
        REM in fact installed and its UI running.
        if not defined DOCKER_DESKTOP_EXE (
            set "DOCKER_DESKTOP_EXE=%ProgramFiles%\Docker\Docker\Docker Desktop.exe"
            if not exist "!DOCKER_DESKTOP_EXE!" set "DOCKER_DESKTOP_EXE=%LOCALAPPDATA%\Programs\DockerDesktop\Docker Desktop.exe"
            if not exist "!DOCKER_DESKTOP_EXE!" set "DOCKER_DESKTOP_EXE=%LOCALAPPDATA%\Programs\Docker\Docker\Docker Desktop.exe"
        )
        if not exist "!DOCKER_DESKTOP_EXE!" (
            echo Couldn't find Docker Desktop. Looked in:
            echo   %ProgramFiles%\Docker\Docker\
            echo   %LOCALAPPDATA%\Programs\DockerDesktop\
            echo   %LOCALAPPDATA%\Programs\Docker\Docker\
            echo Set DOCKER_DESKTOP_EXE if it's installed somewhere else, or install it:
            echo   https://www.docker.com/products/docker-desktop/
            echo.
            pause
            exit /b 1
        )
        REM NOTE: a *running* Docker Desktop UI does not mean a working daemon.
        REM The Linux engine lives in the `docker-desktop` WSL2 distro, so if
        REM WSL is wedged (`wsl -l -v` shows it Stopped), `docker info` fails
        REM and `docker ps` returns a 500 from the dockerDesktopLinuxEngine
        REM pipe even with every Docker Desktop process alive. Fix WSL first --
        REM `wsl --shutdown`, then restart Docker Desktop.
        start "" "!DOCKER_DESKTOP_EXE!"

        REM Cold start (WSL2 utility VM + engine) commonly takes 30s-2min, so
        REM this polls docker info rather than a single blind retry -- a
        REM retry right after `start` would just hit the same npipe error.
        echo Waiting for the Docker daemon to come up ^(cold start can take a couple minutes^)...
        set WAITED=0
        :wait_for_docker
        docker info >nul 2>&1
        if not errorlevel 1 goto docker_ready
        set /a WAITED+=5
        if !WAITED! GEQ 180 (
            echo Docker still isn't responding after 180s. Open Docker Desktop directly
            echo and check its status ^(Settings / whale icon in the system tray^), then
            echo re-run this.
            echo.
            pause
            exit /b 1
        )
        ping -n 6 127.0.0.1 >nul
        goto wait_for_docker
        :docker_ready
        echo Docker daemon is up.
    )

    echo Starting OpenVLA policy server in Docker on http://localhost:8000 ...
    echo First run downloads the model weights -- wait for "Uvicorn running" before enabling Policy Control.
    docker compose --profile openvla up --build
) else (
    echo Unknown mode "%MODE%". Usage: run_server.bat [stub^|openvla]
    exit /b 1
)
