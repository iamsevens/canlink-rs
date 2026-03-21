# Quick Release Guide

## Scope

The current workspace contains 5 publishable crates:

- `canlink-hal`
- `canlink-tscan-sys`
- `canlink-mock`
- `canlink-tscan`
- `canlink-cli`

Recommended publish order:

1. `canlink-hal`
2. `canlink-tscan-sys`
3. `canlink-mock`
4. `canlink-tscan`
5. `canlink-cli`

## Option 1: GitHub Actions

Run `Release Dry Run` first, then `Release Publish`.

- `Release Dry Run`: runs `cargo publish --dry-run` for all crates
- `Release Publish`: publishes crates to crates.io in dependency order and waits for indexing

This is the safest path for a first public release because the workflow already applies the required `patch.crates-io` overrides for unpublished internal dependencies.

## Option 2: Local Scripts

Linux/macOS:

```bash
./scripts/release.sh <version>
```

Windows:

```cmd
scripts\release.bat <version>
```

Example:

```bash
./scripts/release.sh 0.3.0
scripts\release.bat 0.3.0
```

The scripts will:

- run tests and quality checks
- build documentation
- prompt you to update the workspace version and `CHANGELOG.md`
- create a commit and tag
- optionally push to the remote
- publish all 5 crates in order and wait for crates.io indexing

## Option 3: Manual Publish

### 1. Run checks

```bash
cargo test --all-features --workspace

# Linux/macOS
./scripts/check.sh

# Windows
scripts\check.bat

cargo doc --no-deps --all-features --workspace
```

### 2. Update the workspace version

Edit the workspace root `Cargo.toml`:

```toml
[workspace.package]
version = "<version>"
```

### 3. Update CHANGELOG

Make sure `CHANGELOG.md` includes the release notes for the target version.

### 4. Commit and tag

```bash
git add -A
git commit -m "chore: prepare release v<version>"
git tag -a v<version> -m "Release v<version>"
git push origin main
git push origin v<version>
```

### 5. Publish to crates.io

```bash
cd canlink-hal
cargo publish --dry-run --locked
cargo publish --locked

cd ../canlink-tscan-sys
cargo publish --dry-run --locked
cargo publish --locked

cd ../canlink-mock
cargo publish --dry-run --locked
cargo publish --locked

cd ../canlink-tscan
cargo publish --dry-run --locked
cargo publish --locked

cd ../canlink-cli
cargo publish --dry-run --locked
cargo publish --locked
```

Wait for crates.io indexing after each crate before publishing the next one.

### 6. Verify the release

```bash
open https://crates.io/crates/canlink-hal
open https://crates.io/crates/canlink-tscan-sys
open https://crates.io/crates/canlink-mock
open https://crates.io/crates/canlink-tscan
open https://crates.io/crates/canlink-cli

cargo install canlink-cli
canlink --version
```

## Pre-release Checklist

- [ ] all tests pass
- [ ] quality checks pass
- [ ] documentation builds successfully
- [ ] workspace version is updated
- [ ] `CHANGELOG.md` is updated
- [ ] examples still run
- [ ] `README.md` is updated
- [ ] `Release Dry Run` passes
