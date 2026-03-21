@echo off
setlocal EnableExtensions EnableDelayedExpansion
chcp 65001 >nul

set "RUN_HW=1"
set "LOG_DIR="
set "HAS_FAIL=0"

if /i "%~1"=="--help" goto :usage
if /i "%~1"=="--no-hw" (
    set "RUN_HW=0"
    shift
)
if not "%~1"=="" (
    set "LOG_DIR=%~1"
)

if "%LOG_DIR%"=="" (
    for /f %%i in ('powershell -NoProfile -Command "(Get-Date).ToString(\"yyyyMMdd_HHmmss\")"') do (
        set "TS=%%i"
    )
    set "LOG_DIR=_logs\hw_regression\!TS!"
)

rem Runtime DLL is copied by canlink-tscan-sys/build.rs to target output dirs.
rem Do not force PATH to TSMaster install dir, to avoid DLL/Lib mismatch.

mkdir "%LOG_DIR%" >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Failed to create log dir: %LOG_DIR%
    exit /b 1
)

set "SUMMARY=%LOG_DIR%\summary.txt"
echo TSCan HW Regression Summary> "%SUMMARY%"
echo Log dir: !LOG_DIR!>> "%SUMMARY%"
echo Run hw: !RUN_HW!>> "%SUMMARY%"
echo.>> "%SUMMARY%"

echo [INFO] Log directory: !LOG_DIR!
echo [INFO] Start regression...

call :run_step "Build daemon binaries" "cargo build -p canlink-tscan --bin canlink-tscan-daemon --bin canlink-tscan-daemon-stub" "!LOG_DIR!\01_build.log"
call :run_step "Package tests (canlink-tscan)" "cargo test -p canlink-tscan" "!LOG_DIR!\02_test.log"

if "!RUN_HW!"=="1" (
    call :run_step "Hardware example: backend_test" "cargo run -p canlink-tscan --example backend_test" "!LOG_DIR!\03_backend_test.log"
    call :run_step "Hardware example: canfd_test" "cargo run -p canlink-tscan --example canfd_test" "!LOG_DIR!\04_canfd_test.log"
    call :run_step "Hardware example: hardware_filter_test" "cargo run -p canlink-tscan --example hardware_filter_test" "!LOG_DIR!\05_hardware_filter_test.log"
) else (
    echo [SKIP] Hardware examples skipped (--no-hw^)
    echo Hardware example: backend_test: SKIP>> "%SUMMARY%"
    echo Hardware example: canfd_test: SKIP>> "%SUMMARY%"
    echo Hardware example: hardware_filter_test: SKIP>> "%SUMMARY%"
)

echo.>> "%SUMMARY%"
if not "!HAS_FAIL!"=="0" (
    echo FINAL RESULT: FAIL>> "%SUMMARY%"
    echo [RESULT] FAIL. Check logs under !LOG_DIR!
    exit /b 1
) else (
    echo FINAL RESULT: PASS>> "%SUMMARY%"
    echo [RESULT] PASS. Logs are in !LOG_DIR!
    exit /b 0
)

:run_step
set "STEP_NAME=%~1"
set "STEP_CMD=%~2"
set "STEP_LOG=%~3"

echo [RUN] !STEP_NAME!
echo ==== !STEP_NAME! ==== > "!STEP_LOG!"
echo CMD: !STEP_CMD!>> "!STEP_LOG!"
echo.>> "!STEP_LOG!"

cmd /c "!STEP_CMD!" >> "!STEP_LOG!" 2>&1
set "RC=!ERRORLEVEL!"

if not "!RC!"=="0" (
    echo [FAIL] !STEP_NAME! exit=!RC!
    echo !STEP_NAME!: FAIL exit=!RC!>> "%SUMMARY%"
    set "HAS_FAIL=1"
) else (
    echo [PASS] !STEP_NAME!
    echo !STEP_NAME!: PASS>> "%SUMMARY%"
)
exit /b 0

:usage
echo Usage:
echo   scripts\tscan_hw_regression.bat [--no-hw] [log_dir]
echo.
echo Examples:
echo   scripts\tscan_hw_regression.bat
echo   scripts\tscan_hw_regression.bat --no-hw
echo   scripts\tscan_hw_regression.bat _logs\hw_regression\manual
exit /b 0
