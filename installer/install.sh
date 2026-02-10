#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VERSION="3.42.0"

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Vortex Marine Limited - GPS Studio Installer          ║${NC}"
echo -e "${BLUE}║                     Version ${VERSION}                        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo

# Check root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: This installer must be run with sudo${NC}"
    echo "  Usage: sudo bash $0"
    exit 1
fi

# Determine the real user (not root)
REAL_USER="${SUDO_USER:-$USER}"
REAL_HOME=$(eval echo "~$REAL_USER")

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    echo -e "${YELLOW}Detected OS: $NAME $VERSION_ID${NC}"
else
    echo -e "${RED}Error: Cannot detect OS version${NC}"
    exit 1
fi

if [[ "$NAME" != *"Ubuntu"* ]]; then
    echo -e "${YELLOW}Warning: This installer is designed for Ubuntu 22.04/24.04. Proceed with caution.${NC}"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Step 1: Install dependencies
echo
echo -e "${BLUE}[1/5] Installing system dependencies...${NC}"
apt-get update -qq
apt-get install -y \
    libc6 \
    libwebkit2gtk-4.1-0 \
    libgtk-3-0
echo -e "${GREEN}✓ Dependencies installed${NC}"

# Step 2: Install GPS Studio .deb
echo
echo -e "${BLUE}[2/5] Installing GPS Studio...${NC}"
dpkg -i "$SCRIPT_DIR"/gps-studio_*.deb || true
apt-get install -f -y
echo -e "${GREEN}✓ GPS Studio installed${NC}"

# Step 3: Configure udev rules for GPS hardware
echo
echo -e "${BLUE}[3/5] Configuring GPS hardware access...${NC}"
if [ -f "$SCRIPT_DIR/99-scout-gps.rules" ]; then
    cp "$SCRIPT_DIR/99-scout-gps.rules" /etc/udev/rules.d/
    udevadm control --reload-rules
    udevadm trigger
    echo -e "${GREEN}✓ udev rules installed (u-blox, Prolific, FTDI, SiLabs, QinHeng, SiRF, GlobalSat)${NC}"
else
    echo -e "${YELLOW}Warning: udev rules file not found, skipping${NC}"
fi

# Step 4: Add user to dialout group for serial port access
echo
echo -e "${BLUE}[4/5] Configuring serial port access...${NC}"
if id -nG "$REAL_USER" | grep -qw "dialout"; then
    echo -e "${GREEN}✓ User '$REAL_USER' already in dialout group${NC}"
else
    usermod -a -G dialout "$REAL_USER"
    echo -e "${GREEN}✓ User '$REAL_USER' added to dialout group${NC}"
fi

# Step 5: Create application directories
echo
echo -e "${BLUE}[5/5] Setting up application directories...${NC}"
mkdir -p "$REAL_HOME/.config/gps-studio"
chown "$REAL_USER:$REAL_USER" "$REAL_HOME/.config/gps-studio"
mkdir -p "$REAL_HOME/gps-studio-results"
chown "$REAL_USER:$REAL_USER" "$REAL_HOME/gps-studio-results"
echo -e "${GREEN}✓ Config directory: ~/.config/gps-studio${NC}"
echo -e "${GREEN}✓ Results directory: ~/gps-studio-results${NC}"

# Copy uninstaller to a known location
if [ -f "$SCRIPT_DIR/uninstall.sh" ]; then
    cp "$SCRIPT_DIR/uninstall.sh" /usr/local/bin/gps-studio-uninstall
    chmod +x /usr/local/bin/gps-studio-uninstall
fi

# Done
echo
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║           GPS Studio ${VERSION} Installed Successfully!       ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo
echo -e "${YELLOW}IMPORTANT: Log out and log back in for serial port access${NC}"
echo -e "${YELLOW}           (dialout group membership takes effect on next login)${NC}"
echo
echo -e "${BLUE}To launch:${NC}  gps-studio"
echo -e "${BLUE}To remove:${NC}  sudo gps-studio-uninstall"
echo
