# Scout GPS Test

Factory GPS hardware verification tool for Scout Tablets. Tests GPS receiver performance against configurable specifications and generates pass/fail reports.

## Features

- **Auto-detection**: Scans serial ports and identifies GPS hardware (u-blox, SiRF, generic NMEA)
- **Live monitoring**: Real-time satellite signals, SNR, DOP values, constellation tracking
- **Pass/fail testing**: Configurable criteria with stability verification
- **u-blox optimization**: Automatic multi-constellation configuration (GPS + GLONASS + SBAS) for u-blox receivers
- **Generic support**: Works with any NMEA 0183 GPS receiver
- **Factory reports**: JSON test reports saved per device for traceability

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

Criteria are configurable via the Config button or by editing `~/.config/scout-gps-test/criteria.json`.

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

Output: `src-tauri/target/release/bundle/deb/scout-gps-test_1.0.0_amd64.deb`

## Installation (Factory)

```bash
# Install the .deb package
sudo dpkg -i scout-gps-test_1.0.0_amd64.deb

# Grant serial port access (one-time, requires logout/login)
sudo usermod -a -G dialout $USER

# Optional: install udev rules for broader device support
sudo cp 99-scout-gps.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules

# Create results directory
mkdir -p ~/scout-gps-results

# Launch
scout-gps-test
```

## Factory Workflow

1. Connect tablet to test station
2. Launch Scout GPS Test (auto-detects GPS hardware)
3. Verify device identity (manufacturer, serial number)
4. Press **START TEST**
5. Wait for **PASS** or **FAIL** verdict
6. Press **Save Report** to record results
7. Press **Next Tablet** to reset for next unit

## Test Reports

Reports are saved as JSON to `~/scout-gps-results/` with filename format:
`gps-test_{serial}_{timestamp}.json`

## Development

```bash
npm install
cargo tauri dev
```

## Tech Stack

- **Backend**: Rust + Tauri 2.0
- **Frontend**: React 19 + TypeScript + Vite
- **GPS**: serialport crate + nmea crate
- **Protocol**: NMEA 0183, u-blox UBX (optional)
