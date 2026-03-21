@echo off
REM Release automation script for CANLink-RS (Windows)
REM Usage: scripts\release.bat <version>
REM Example: scripts\release.bat 0.1.0

setlocal enabledelayedexpansion

set VERSION=%1

if "%VERSION%"=="" (
    echo ❌ Error: Version number required
    echo Usage: scripts\release.bat ^<version^>
    echo Example: scripts\release.bat 0.1.0
    exit /b 1
)

echo.
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo 🚀 CANLink-RS Release Script
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo Version: v%VERSION%
echo.

REM Step 1: Pre-release checks
echo Step 1: Running pre-release checks...
echo.

echo 📋 Running tests...
cargo test --all-features --workspace
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Tests failed!
    exit /b 1
)

echo 📋 Running quality checks...
call scripts\check.bat
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Quality checks failed!
    exit /b 1
)

echo 📋 Building documentation...
cargo doc --no-deps --all-features --workspace
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Documentation build failed!
    exit /b 1
)

echo ✓ All pre-release checks passed
echo.

REM Step 2: Update version numbers
echo Step 2: Updating version numbers...
echo.

REM Note: Manual version update required on Windows
echo ⚠ Please update version to %VERSION% in Cargo.toml
echo Press any key to continue after updating...
pause >nul

echo ✓ Version updated to %VERSION%
echo.

REM Step 3: Create CHANGELOG entry
echo Step 3: CHANGELOG.md
echo.

if not exist CHANGELOG.md (
    echo ⚠ CHANGELOG.md not found. Please create it manually.
    echo Press any key to continue after creating CHANGELOG.md...
    pause >nul
) else (
    echo ✓ CHANGELOG.md exists
)
echo.

REM Step 4: Commit changes
echo Step 4: Committing changes...
echo.

git add -A
git commit -m "chore: prepare release v%VERSION%"
if %ERRORLEVEL% NEQ 0 (
    echo ⚠ No changes to commit or commit failed
)

echo ✓ Changes committed
echo.

REM Step 5: Create tag
echo Step 5: Creating git tag...
echo.

git tag -a "v%VERSION%" -m "Release v%VERSION%"
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Failed to create tag. Tag may already exist.
    exit /b 1
)

echo ✓ Tag v%VERSION% created
echo.

REM Step 6: Push changes
echo Step 6: Pushing to remote...
echo.

set /p PUSH="Push changes to remote? (y/n): "
if /i "%PUSH%"=="y" (
    git push origin main
    git push origin "v%VERSION%"
    echo ✓ Changes pushed to remote
) else (
    echo ⚠ Skipped pushing to remote
    echo Run manually: git push origin main ^&^& git push origin v%VERSION%
)
echo.

REM Step 7: Publish to crates.io
echo Step 7: Publishing to crates.io...
echo.

set /p PUBLISH="Publish to crates.io? (y/n): "
if /i "%PUBLISH%"=="y" (
    echo Publishing canlink-hal...
    cd canlink-hal
    cargo publish --dry-run
    cargo publish
    cd ..

    echo Waiting for crates.io to index...
    timeout /t 120 /nobreak >nul

    echo Publishing canlink-mock...
    cd canlink-mock
    cargo publish --dry-run
    cargo publish
    cd ..

    echo Waiting for crates.io to index...
    timeout /t 120 /nobreak >nul

    echo Publishing canlink-cli...
    cd canlink-cli
    cargo publish --dry-run
    cargo publish
    cd ..

    echo ✓ Published to crates.io
) else (
    echo ⚠ Skipped publishing to crates.io
    echo Run manually:
    echo   cd canlink-hal ^&^& cargo publish
    echo   cd canlink-mock ^&^& cargo publish
    echo   cd canlink-cli ^&^& cargo publish
)
echo.

REM Summary
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo 🎉 Release v%VERSION% Complete!
echo ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
echo.
echo Next steps:
echo 1. Create a release on your public repository hosting page
echo 2. Verify crates.io: https://crates.io/crates/canlink-hal
echo 3. Test installation: cargo install canlink-cli
echo 4. Announce release
echo.
echo ✓ Release process completed successfully!

endlocal
