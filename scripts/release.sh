#!/usr/bin/env bash
# Release automation script for CANLink-RS
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.1.0

set -e

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "❌ Error: Version number required"
    echo "Usage: ./scripts/release.sh <version>"
    echo "Example: ./scripts/release.sh 0.1.0"
    exit 1
fi

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 CANLink-RS Release Script"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Version: v$VERSION"
echo ""

# Step 1: Pre-release checks
echo -e "${BLUE}Step 1: Running pre-release checks...${NC}"
echo ""

echo "📋 Running tests..."
cargo test --all-features --workspace || {
    echo -e "${RED}❌ Tests failed!${NC}"
    exit 1
}

echo "📋 Running quality checks..."
./scripts/check.sh || {
    echo -e "${RED}❌ Quality checks failed!${NC}"
    exit 1
}

echo "📋 Building documentation..."
cargo doc --no-deps --all-features --workspace || {
    echo -e "${RED}❌ Documentation build failed!${NC}"
    exit 1
}

echo -e "${GREEN}✓ All pre-release checks passed${NC}"
echo ""

# Step 2: Update version numbers
echo -e "${BLUE}Step 2: Updating version numbers...${NC}"
echo ""

# Update workspace Cargo.toml
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
rm Cargo.toml.bak

echo -e "${GREEN}✓ Version updated to $VERSION${NC}"
echo ""

# Step 3: Create CHANGELOG entry
echo -e "${BLUE}Step 3: CHANGELOG.md${NC}"
echo ""

if [ ! -f CHANGELOG.md ]; then
    echo -e "${YELLOW}⚠ CHANGELOG.md not found. Please create it manually.${NC}"
    echo "Press Enter to continue after creating CHANGELOG.md..."
    read
else
    echo -e "${GREEN}✓ CHANGELOG.md exists${NC}"
fi
echo ""

# Step 4: Commit changes
echo -e "${BLUE}Step 4: Committing changes...${NC}"
echo ""

git add -A
git commit -m "chore: prepare release v$VERSION

- Update version to $VERSION
- Update CHANGELOG.md
- Update documentation
" || {
    echo -e "${YELLOW}⚠ No changes to commit${NC}"
}

echo -e "${GREEN}✓ Changes committed${NC}"
echo ""

# Step 5: Create tag
echo -e "${BLUE}Step 5: Creating git tag...${NC}"
echo ""

git tag -a "v$VERSION" -m "Release v$VERSION

See CHANGELOG.md for details.
" || {
    echo -e "${RED}❌ Failed to create tag. Tag may already exist.${NC}"
    exit 1
}

echo -e "${GREEN}✓ Tag v$VERSION created${NC}"
echo ""

# Step 6: Push changes
echo -e "${BLUE}Step 6: Pushing to remote...${NC}"
echo ""

echo "Push changes to remote? (y/n)"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    git push origin main
    git push origin "v$VERSION"
    echo -e "${GREEN}✓ Changes pushed to remote${NC}"
else
    echo -e "${YELLOW}⚠ Skipped pushing to remote${NC}"
    echo "Run manually: git push origin main && git push origin v$VERSION"
fi
echo ""

# Step 7: Publish to crates.io
echo -e "${BLUE}Step 7: Publishing to crates.io...${NC}"
echo ""

echo "Publish to crates.io? (y/n)"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    echo "Publishing canlink-hal..."
    cd canlink-hal
    cargo publish --dry-run
    cargo publish
    cd ..

    echo "Waiting for crates.io to index..."
    sleep 120

    echo "Publishing canlink-mock..."
    cd canlink-mock
    cargo publish --dry-run
    cargo publish
    cd ..

    echo "Waiting for crates.io to index..."
    sleep 120

    echo "Publishing canlink-cli..."
    cd canlink-cli
    cargo publish --dry-run
    cargo publish
    cd ..

    echo -e "${GREEN}✓ Published to crates.io${NC}"
else
    echo -e "${YELLOW}⚠ Skipped publishing to crates.io${NC}"
    echo "Run manually:"
    echo "  cd canlink-hal && cargo publish"
    echo "  cd canlink-mock && cargo publish"
    echo "  cd canlink-cli && cargo publish"
fi
echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎉 Release v$VERSION Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Next steps:"
echo "1. Create a release on your public repository hosting page"
echo "2. Verify crates.io: https://crates.io/crates/canlink-hal"
echo "3. Test installation: cargo install canlink-cli"
echo "4. Announce release"
echo ""
echo -e "${GREEN}✓ Release process completed successfully!${NC}"
