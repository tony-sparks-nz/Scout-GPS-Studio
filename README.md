# Vortex Marine Limited - GPS Studio

GPS hardware verification and analysis tool. Tests GPS receiver performance against configurable specifications and generates pass/fail reports.

## Features

- **Auto-detection**: Scans serial ports and identifies GPS hardware (u-blox, SiRF, generic NMEA)
- **Live monitoring**: Real-time satellite signals, SNR, DOP values, constellation tracking
- **Pass/fail testing**: Configurable criteria with stability verification
- **u-blox optimization**: Automatic multi-constellation configuration (GPS + GLONASS + SBAS) for u-blox receivers
- **Generic support**: Works with any NMEA 0183 GPS receiver
- **Hardware debug**: Full USB device identity, signal statistics, per-constellation breakdown
- **Map view**: GPS fix location with multiple basemaps (Dark, Light, Voyager, Satellite)
- **Reports**: JSON test reports saved per device for traceability

## Test Criteria (Defaults)

| Criterion | Threshold |
|---|---|
| Min satellites | 6 |
| Max HDOP | 2.0 |
| Max PDOP | 3.0 |
| Min avg SNR | 25.0 dB |
| Min strong sats (SNR>=30) | 4 |
| Max time to first fix | 60s |
| Min constellations | 2 |
| Stability duration | 10s |

Criteria are configurable via the Config button or by editing `~/.config/gps-studio/criteria.json`.

## Building

### Prerequisites (Ubuntu)

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  libudev-dev pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build

```bash
npm install
cargo tauri build
```

Output: `src-tauri/target/release/bundle/deb/gps-studio_3.43.0_amd64.deb`

## Installation

```bash
# Install the .deb package
sudo dpkg -i gps-studio_3.43.0_amd64.deb

# Grant serial port access (one-time, requires logout/login)
sudo usermod -a -G dialout $USER

# Optional: install udev rules for broader device support
sudo cp 99-scout-gps.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules

# Create results directory
mkdir -p ~/gps-studio-results

# Launch
gps-studio
```

## Workflow

1. Connect GPS device
2. Launch GPS Studio (auto-detects GPS hardware)
3. Verify device identity (manufacturer, serial number, VID/PID)
4. Press **START TEST**
5. Wait for **PASS** or **FAIL** verdict
6. Press **Save Report** to record results
7. Press **Next Tablet** to reset for next unit

## Test Reports

Reports are saved as JSON to `~/gps-studio-results/` with filename format:
`gps-test_{serial}_{timestamp}.json`

## Development

```bash
npm install
cargo tauri dev
```

## Tech Stack

- **Backend**: Rust + Tauri 2.0
- **Frontend**: React 19 + TypeScript + Vite + Leaflet
- **GPS**: serialport crate + nmea crate
- **Protocol**: NMEA 0183, u-blox UBX (optional)
