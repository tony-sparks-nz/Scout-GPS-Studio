#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Vortex Marine Limited - GPS Studio Uninstaller        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo

# Check root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: This uninstaller must be run with sudo${NC}"
    echo "  Usage: sudo gps-studio-uninstall"
    exit 1
fi

REAL_USER="${SUDO_USER:-$USER}"
REAL_HOME=$(eval echo "~$REAL_USER")

# Remove the package
echo -e "${BLUE}Removing GPS Studio...${NC}"
if dpkg -l | grep -q gps-studio; then
    dpkg --purge gps-studio
    echo -e "${GREEN}✓ GPS Studio removed${NC}"
else
    echo -e "${YELLOW}GPS Studio package not found${NC}"
fi

# Remove udev rules
echo -e "${BLUE}Removing udev rules...${NC}"
if [ -f /etc/udev/rules.d/99-scout-gps.rules ]; then
    rm /etc/udev/rules.d/99-scout-gps.rules
    udevadm control --reload-rules
    echo -e "${GREEN}✓ udev rules removed${NC}"
else
    echo -e "${YELLOW}udev rules not found${NC}"
fi

# Ask about user data
echo
echo -e "${YELLOW}Remove user data?${NC}"
echo "  Config:  $REAL_HOME/.config/gps-studio"
echo "  Results: $REAL_HOME/gps-studio-results"
echo
read -p "Remove user data? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf "$REAL_HOME/.config/gps-studio"
    rm -rf "$REAL_HOME/gps-studio-results"
    echo -e "${GREEN}✓ User data removed${NC}"
else
    echo -e "${BLUE}User data preserved${NC}"
fi

# Remove this uninstaller
rm -f /usr/local/bin/gps-studio-uninstall

echo
echo -e "${GREEN}✓ GPS Studio uninstalled${NC}"
echo
