#!/usr/bin/env bash
# Release automation script for CANLink-RS
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.3.0

set -euo pipefail

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
    echo "Error: version number required" >&2
    echo "Usage: ./scripts/release.sh <version>" >&2
    echo "Example: ./scripts/release.sh 0.3.0" >&2
    exit 1
fi

wait_for_crate_version() {
    local crate="$1"
    local version="$2"
    local line=""

    for attempt in $(seq 1 30); do
        line="$(cargo search "$crate" --limit 1 2>/dev/null || true)"
        echo "Waiting for ${crate} ${version} to be indexed... attempt ${attempt}/30"
        if printf '%s\n' "$line" | grep -F "${crate} = \"${version}\"" >/dev/null; then
            echo "${crate} ${version} is indexed."
            return 0
        fi
        sleep 20
    done

    echo "Error: timed out waiting for ${crate} ${version} to appear on crates.io." >&2
    return 1
}

publish_crate() {
    local crate="$1"
    local version="$2"

    echo "Publishing ${crate}..."
    cargo publish -p "$crate" --dry-run --locked
    cargo publish -p "$crate" --locked
    wait_for_crate_version "$crate" "$version"
}

echo
echo "========================================"
echo "CANLink-RS Release Script"
echo "========================================"
echo "Version: v${VERSION}"
echo

echo "Step 1: running pre-release checks..."
echo

echo "Running tests..."
cargo test --all-features --workspace

echo "Running quality checks..."
./scripts/check.sh

echo "Building documentation..."
cargo doc --no-deps --all-features --workspace

echo "Pre-release checks passed."
echo

echo "Step 2: update workspace version in Cargo.toml..."
echo "Update [workspace.package].version to ${VERSION}, then press Enter."
read -r

echo "Step 3: verify CHANGELOG.md..."
if [ ! -f CHANGELOG.md ]; then
    echo "Error: CHANGELOG.md not found" >&2
    exit 1
fi
echo "CHANGELOG.md found."
echo

echo "Step 4: committing release preparation..."
git add -A
git commit -m "chore: prepare release v${VERSION}" || echo "Note: no changes were committed."
echo

echo "Step 5: creating git tag..."
git tag -a "v${VERSION}" -m "Release v${VERSION}"
echo "Tag v${VERSION} created."
echo

read -r -p "Push changes to remote? (y/n): " response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    git push origin main
    git push origin "v${VERSION}"
    echo "Remote push completed."
else
    echo "Skipped remote push."
fi
echo

read -r -p "Publish to crates.io? (y/n): " response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    publish_crate canlink-hal "$VERSION"
    publish_crate canlink-tscan-sys "$VERSION"
    publish_crate canlink-mock "$VERSION"
    publish_crate canlink-tscan "$VERSION"
    publish_crate canlink-cli "$VERSION"
    echo "crates.io publish completed."
else
    echo "Skipped crates.io publish."
    echo "Recommended manual order:"
    echo "  canlink-hal"
    echo "  canlink-tscan-sys"
    echo "  canlink-mock"
    echo "  canlink-tscan"
    echo "  canlink-cli"
fi
echo

echo "========================================"
echo "Release flow finished"
echo "========================================"
echo "Verify crates.io pages:"
echo "  https://crates.io/crates/canlink-hal"
echo "  https://crates.io/crates/canlink-tscan-sys"
echo "  https://crates.io/crates/canlink-mock"
echo "  https://crates.io/crates/canlink-tscan"
echo "  https://crates.io/crates/canlink-cli"
echo
echo "Test installation:"
echo "  cargo install canlink-cli"
echo "  canlink --version"
