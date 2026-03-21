@echo off
REM Release automation script for CANLink-RS (Windows)
REM Usage: scripts\release.bat <version>
REM Example: scripts\release.bat 0.3.0

setlocal enabledelayedexpansion

set "VERSION=%~1"

if "%VERSION%"=="" (
    echo Error: version number required
    echo Usage: scripts\release.bat ^<version^>
    echo Example: scripts\release.bat 0.3.0
    exit /b 1
)

echo.
echo ========================================
echo CANLink-RS Release Script
echo ========================================
echo Version: v%VERSION%
echo.

echo Step 1: running pre-release checks...
echo.

echo Running tests...
cargo test --all-features --workspace
if %ERRORLEVEL% NEQ 0 (
    echo Error: tests failed
    exit /b 1
)

echo Running quality checks...
call scripts\check.bat
if %ERRORLEVEL% NEQ 0 (
    echo Error: quality checks failed
    exit /b 1
)

echo Building documentation...
cargo doc --no-deps --all-features --workspace
if %ERRORLEVEL% NEQ 0 (
    echo Error: documentation build failed
    exit /b 1
)

echo Pre-release checks passed.
echo.

echo Step 2: update workspace version in Cargo.toml...
echo Update [workspace.package].version to %VERSION%, then press any key.
pause >nul

echo.
echo Step 3: verify CHANGELOG.md...
if not exist CHANGELOG.md (
    echo Error: CHANGELOG.md not found
    exit /b 1
)
echo CHANGELOG.md found.
echo.

echo Step 4: committing release preparation...
git add -A
git commit -m "chore: prepare release v%VERSION%"
if %ERRORLEVEL% NEQ 0 (
    echo Note: no changes were committed.
)
echo.

echo Step 5: creating git tag...
git tag -a "v%VERSION%" -m "Release v%VERSION%"
if %ERRORLEVEL% NEQ 0 (
    echo Error: failed to create tag. It may already exist.
    exit /b 1
)
echo Tag v%VERSION% created.
echo.

set /p PUSH=Push changes to remote? (y/n): 
if /I "%PUSH%"=="y" (
    git push origin main
    if %ERRORLEVEL% NEQ 0 exit /b 1
    git push origin "v%VERSION%"
    if %ERRORLEVEL% NEQ 0 exit /b 1
    echo Remote push completed.
) else (
    echo Skipped remote push.
)
echo.

set /p PUBLISH=Publish to crates.io? (y/n): 
if /I "%PUBLISH%"=="y" (
    call :publish_crate canlink-hal %VERSION%
    if %ERRORLEVEL% NEQ 0 exit /b 1

    call :publish_crate canlink-tscan-sys %VERSION%
    if %ERRORLEVEL% NEQ 0 exit /b 1

    call :publish_crate canlink-mock %VERSION%
    if %ERRORLEVEL% NEQ 0 exit /b 1

    call :publish_crate canlink-tscan %VERSION%
    if %ERRORLEVEL% NEQ 0 exit /b 1

    call :publish_crate canlink-cli %VERSION%
    if %ERRORLEVEL% NEQ 0 exit /b 1

    echo crates.io publish completed.
) else (
    echo Skipped crates.io publish.
    echo Recommended manual order:
    echo   canlink-hal
    echo   canlink-tscan-sys
    echo   canlink-mock
    echo   canlink-tscan
    echo   canlink-cli
)
echo.

echo ========================================
echo Release flow finished

echo ========================================
echo Verify crates.io pages:
echo   https://crates.io/crates/canlink-hal
echo   https://crates.io/crates/canlink-tscan-sys
echo   https://crates.io/crates/canlink-mock
echo   https://crates.io/crates/canlink-tscan
echo   https://crates.io/crates/canlink-cli
echo.
echo Test installation:
echo   cargo install canlink-cli
echo   canlink --version

goto :eof

:publish_crate
set "CRATE=%~1"
set "VER=%~2"
echo Publishing %CRATE%...
cargo publish -p %CRATE% --dry-run --locked
if %ERRORLEVEL% NEQ 0 (
    echo Error: dry-run failed for %CRATE%
    exit /b 1
)
cargo publish -p %CRATE% --locked
if %ERRORLEVEL% NEQ 0 (
    echo Error: publish failed for %CRATE%
    exit /b 1
)
call :wait_for_crate_version %CRATE% %VER%
if %ERRORLEVEL% NEQ 0 exit /b 1
exit /b 0

:wait_for_crate_version
set "CRATE=%~1"
set "VER=%~2"
for /l %%I in (1,1,30) do (
    set "SEARCH_LINE="
    for /f "usebackq delims=" %%L in (`cargo search %CRATE% --limit 1 2^>nul`) do set "SEARCH_LINE=%%L"
    echo Waiting for %CRATE% %VER% to be indexed... attempt %%I/30
    echo !SEARCH_LINE! | findstr /C:"%CRATE% = \"%VER%\"" >nul
    if not errorlevel 1 (
        echo %CRATE% %VER% is indexed.
        exit /b 0
    )
    timeout /t 20 /nobreak >nul
)
echo Error: timed out waiting for %CRATE% %VER% to appear on crates.io.
exit /b 1
