@echo off
setlocal enabledelayedexpansion

echo ============================================================
echo   🚀 AGILE UP - NEXUS SUITE BUILDER (v2026.4)
echo ============================================================
echo.

set DIST_DIR=dist_nexus_suite
if not exist %DIST_DIR% mkdir %DIST_DIR%

:: 1. Build GitNexus CLI
echo [1/3] Building GitNexus CLI...
cargo build --release -p gitnexus-cli
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Failed to build GitNexus CLI
    exit /b %ERRORLEVEL%
)
copy target\release\gitnexus.exe %DIST_DIR%\gitnexus.exe > nul
echo ✅ GitNexus CLI ready in %DIST_DIR%\gitnexus.exe

:: 2. Build GitNexus Desktop (Tauri)
echo.
echo [2/3] Building GitNexus Desktop Application...
cd crates\gitnexus-desktop\ui
call npm install --silent
call npm run build
cd ..\..\..
cargo tauri build --project crates/gitnexus-desktop
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Failed to build GitNexus Desktop
    exit /b %ERRORLEVEL%
)
:: Move the installer/binary to dist
echo ✅ GitNexus Desktop ready.

:: 3. Build NexusBrain (Tauri)
echo.
echo [3/3] Building NexusBrain (Knowledge IDE)...
cd nexus-brain
call npm install --silent
call npm run build
cd ..
cargo tauri build --project nexus-brain/src-tauri
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Failed to build NexusBrain
    exit /b %ERRORLEVEL%
)
echo ✅ NexusBrain ready.

echo.
echo ============================================================
echo   🎉 NEXUS SUITE BUILT SUCCESSFULLY
echo   Target folder: %DIST_DIR%
echo ============================================================
pause
