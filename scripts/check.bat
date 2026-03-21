@echo off
REM Code Quality Check Script for Windows
REM Run this before committing to ensure code quality

setlocal

echo.
echo 🔍 Running code quality checks...
echo.

set FAILED=0

REM Function to run a check
call :run_check "Rustfmt" "cargo fmt --all -- --check"
call :run_check "Clippy" "cargo clippy --all-targets --all-features -- -D warnings"
call :run_check "Build" "cargo build --all-features"
call :run_check "Tests" "cargo test --all-features"
call :run_check "Doc Tests" "cargo test --doc --all-features"
call :run_check "Documentation" "cargo doc --no-deps --all-features --document-private-items"

REM Check for optional tools
where cargo-audit >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    call :run_check "Security Audit" "cargo audit"
) else (
    echo ⚠ cargo-audit not installed, skipping security audit
    echo   Install with: cargo install cargo-audit
    echo.
)

REM Summary
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo 📊 Summary
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

if %FAILED% EQU 0 (
    echo ✓ All checks passed!
    echo.
    echo You're ready to commit! 🚀
    exit /b 0
) else (
    echo ✗ %FAILED% check^(s^) failed
    echo.
    echo Please fix the issues before committing.
    exit /b 1
)

:run_check
set CHECK_NAME=%~1
set CHECK_CMD=%~2

echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo 📋 %CHECK_NAME%
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

call %CHECK_CMD%
if %ERRORLEVEL% EQU 0 (
    echo ✓ %CHECK_NAME% passed
) else (
    echo ✗ %CHECK_NAME% failed
    set /a FAILED+=1
)
echo.
goto :eof
