#!/bin/bash

# Script to bump version and create git tag for wirerust
# Usage: ./scripts/bump-version.sh [patch|minor|major]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "This script must be run from the project root directory"
    exit 1
fi

# Check if git is available
if ! command -v git &> /dev/null; then
    print_error "git is required but not installed"
    exit 1
fi

# Check if we have uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    print_warning "You have uncommitted changes. Please commit or stash them before bumping version."
    git status --short
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | cut -d '"' -f2)
print_info "Current version: $CURRENT_VERSION"

# Parse version components
IFS='.' read -ra VERSION_PARTS <<< "$CURRENT_VERSION"
MAJOR=${VERSION_PARTS[0]}
MINOR=${VERSION_PARTS[1]}
PATCH=${VERSION_PARTS[2]}

# Determine version bump type
BUMP_TYPE=${1:-patch}

case $BUMP_TYPE in
    patch)
        NEW_PATCH=$((PATCH + 1))
        NEW_VERSION="$MAJOR.$MINOR.$NEW_PATCH"
        print_info "Bumping patch version: $CURRENT_VERSION -> $NEW_VERSION"
        ;;
    minor)
        NEW_MINOR=$((MINOR + 1))
        NEW_VERSION="$MAJOR.$NEW_MINOR.0"
        print_info "Bumping minor version: $CURRENT_VERSION -> $NEW_VERSION"
        ;;
    major)
        NEW_MAJOR=$((MAJOR + 1))
        NEW_VERSION="$NEW_MAJOR.0.0"
        print_info "Bumping major version: $CURRENT_VERSION -> $NEW_VERSION"
        ;;
    *)
        print_error "Invalid bump type: $BUMP_TYPE. Use patch, minor, or major."
        exit 1
        ;;
esac

# Update version in Cargo.toml
print_info "Updating Cargo.toml..."
sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Update version in Cargo.lock (if it exists)
if [ -f "Cargo.lock" ]; then
    print_info "Updating Cargo.lock..."
    sed -i.bak "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.lock
    rm Cargo.lock.bak
fi

# Commit the version change
print_info "Committing version change..."
git add Cargo.toml Cargo.lock
git commit -m "Bump version to $NEW_VERSION"

# Create and push git tag
TAG_NAME="v$NEW_VERSION"
print_info "Creating git tag: $TAG_NAME"
git tag -a "$TAG_NAME" -m "Release $NEW_VERSION"

# Ask user if they want to push
read -p "Push changes and tag to remote? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    print_info "Pushing changes..."
    git push origin main
    print_info "Pushing tag..."
    git push origin "$TAG_NAME"
    print_info "Version $NEW_VERSION has been released!"
    print_info "GitHub Actions will automatically publish to crates.io"
else
    print_warning "Changes committed but not pushed. Run the following commands manually:"
    echo "  git push origin main"
    echo "  git push origin $TAG_NAME"
fi

print_info "Version bump complete!" 