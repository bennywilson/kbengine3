@echo off
REM Build + serve the splat demo's browser (wasm) build.
REM
REM Thin wrapper around the repo's single build system, launcher\build.py, so
REM this example keeps the build_wasm.bat the README refers to without a second
REM (drift-prone) copy of the cargo / wasm-bindgen / asset-copy steps living
REM here. build.py is the single source of truth; this just invokes it.
REM
REM It compiles to wasm, runs wasm-bindgen, copies index.html + the runtime
REM assets into the serve directory, and serves on the splat demo's assigned
REM port (http://127.0.0.1:8003) with no-cache headers until you press Ctrl+C.
REM
REM Re-run this any time you edit index.html: the served copy lives in
REM target\wasm32-unknown-unknown\release and is only refreshed by a build, so
REM an edit to this source index.html is invisible in the browser until then.
REM
REM The policy server (see policy_server\run_server.bat) runs separately on
REM port 8000; the two don't collide.
cd /d "%~dp0..\.."
python launcher\build.py run splat wasm
