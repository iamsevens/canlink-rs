#!/usr/bin/env bash
# Code Quality Check Script
# Run this before committing to ensure code quality

set -e

echo "🔍 Running code quality checks..."
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track failures
FAILED=0

# Function to run a check
run_check() {
    local name=$1
    local command=$2

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📋 $name"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if eval "$command"; then
        echo -e "${GREEN}✓ $name passed${NC}"
    else
        echo -e "${RED}✗ $name failed${NC}"
        FAILED=$((FAILED + 1))
    fi
    echo ""
}

# 1. Check formatting
run_check "Rustfmt" "cargo fmt --all -- --check"

# 2. Run Clippy
run_check "Clippy" "cargo clippy --all-targets --all-features -- -D warnings"

# 3. Build all crates
run_check "Build" "cargo build --all-features"

# 4. Run tests
run_check "Tests" "cargo test --all-features"

# 5. Run doc tests
run_check "Doc Tests" "cargo test --doc --all-features"

# 6. Check documentation
run_check "Documentation" "cargo doc --no-deps --all-features --document-private-items"

# 7. Check for unused dependencies
if command -v cargo-udeps &> /dev/null; then
    run_check "Unused Dependencies" "cargo +nightly udeps --all-features"
else
    echo -e "${YELLOW}⚠ cargo-udeps not installed, skipping unused dependencies check${NC}"
    echo "  Install with: cargo install cargo-udeps"
    echo ""
fi

# 8. Security audit
if command -v cargo-audit &> /dev/null; then
    run_check "Security Audit" "cargo audit"
else
    echo -e "${YELLOW}⚠ cargo-audit not installed, skipping security audit${NC}"
    echo "  Install with: cargo install cargo-audit"
    echo ""
fi

# 9. Check for outdated dependencies
if command -v cargo-outdated &> /dev/null; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📦 Outdated Dependencies"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    cargo outdated --root-deps-only
    echo ""
else
    echo -e "${YELLOW}⚠ cargo-outdated not installed, skipping outdated dependencies check${NC}"
    echo "  Install with: cargo install cargo-outdated"
    echo ""
fi

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    echo ""
    echo "You're ready to commit! 🚀"
    exit 0
else
    echo -e "${RED}✗ $FAILED check(s) failed${NC}"
    echo ""
    echo "Please fix the issues before committing."
    exit 1
fi
