#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     GPS Studio - Build Self-Extracting Installer          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo

# Must run from project root
if [ ! -f "package.json" ]; then
    echo -e "${RED}Error: Must run from project root directory${NC}"
    exit 1
fi

# Extract version from tauri.conf.json
VERSION=$(grep '"version"' src-tauri/tauri.conf.json | head -1 | sed 's/.*"\([0-9.]*\)".*/\1/')
echo -e "${YELLOW}Version: ${VERSION}${NC}"
echo

# Check for makeself
if ! command -v makeself &> /dev/null; then
    echo -e "${YELLOW}Installing makeself...${NC}"
    sudo apt-get install -y makeself
fi

# Check for the .deb
DEB_FILE=$(find src-tauri/target/release/bundle/deb -name "gps-studio_*.deb" 2>/dev/null | head -1)
if [ -z "$DEB_FILE" ]; then
    echo -e "${RED}Error: .deb not found. Run 'cargo tauri build' first.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Found: ${DEB_FILE}${NC}"

# Create staging directory
STAGING="/tmp/gps-studio-installer-staging"
rm -rf "$STAGING"
mkdir -p "$STAGING"

# Copy files to staging
echo -e "${BLUE}Staging installer files...${NC}"
cp "$DEB_FILE" "$STAGING/"
cp 99-scout-gps.rules "$STAGING/"
cp installer/install.sh "$STAGING/"
cp installer/uninstall.sh "$STAGING/"
chmod +x "$STAGING/install.sh"
chmod +x "$STAGING/uninstall.sh"

echo -e "${GREEN}✓ Staged: .deb, udev rules, install script, uninstall script${NC}"
echo

# Build the self-extracting archive
OUTPUT="gps-studio-${VERSION}-installer.run"
echo -e "${BLUE}Building self-extracting installer...${NC}"

makeself --sha256 \
    "$STAGING" \
    "$OUTPUT" \
    "Vortex Marine Limited - GPS Studio v${VERSION} Installer" \
    ./install.sh

# Clean up
rm -rf "$STAGING"

echo
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║              Installer Built Successfully!                ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo
echo -e "${BLUE}Output:${NC}  $(pwd)/${OUTPUT}"
echo -e "${BLUE}Size:${NC}    $(ls -lh "$OUTPUT" | awk '{print $5}')"
echo -e "${BLUE}SHA256:${NC}  $(sha256sum "$OUTPUT" | awk '{print $1}')"
echo
echo -e "${YELLOW}Install on target Ubuntu machine:${NC}"
echo "  sudo bash ${OUTPUT}"
echo
