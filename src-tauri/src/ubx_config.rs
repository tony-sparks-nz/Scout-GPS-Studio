// UBX protocol definitions and marine optimization profiles for u-blox 7 and 8 series
//
// References:
//   u-blox 8/M8 Receiver Description (UBX-13003221)
//   u-blox 7 Receiver Description (GPS.G7-SW-12001)

use serde::{Deserialize, Serialize};

// ============ UBX Protocol Constants ============

pub const UBX_SYNC_1: u8 = 0xB5;
pub const UBX_SYNC_2: u8 = 0x62;

// Message classes
pub const UBX_CLASS_CFG: u8 = 0x06;
pub const UBX_CLASS_MON: u8 = 0x0A;

// Message IDs
pub const UBX_MON_VER: u8 = 0x04;
pub const UBX_CFG_GNSS: u8 = 0x3E;
pub const UBX_CFG_NAV5: u8 = 0x24;
pub const UBX_CFG_RATE: u8 = 0x08;
pub const UBX_CFG_SBAS: u8 = 0x16;
pub const UBX_CFG_MSG: u8 = 0x01;
pub const UBX_CFG_NMEA: u8 = 0x17;
pub const UBX_CFG_CFG: u8 = 0x09;

// NMEA message IDs (under class 0xF0)
const NMEA_GGA: u8 = 0x00;
const NMEA_GLL: u8 = 0x01;
const NMEA_GSA: u8 = 0x02;
const NMEA_GSV: u8 = 0x03;
const NMEA_RMC: u8 = 0x04;
const NMEA_VTG: u8 = 0x05;

// ============ Chip Identification ============

/// Detected u-blox chip series
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UbloxSeries {
    Series7,
    Series8,
    Unknown,
}

impl std::fmt::Display for UbloxSeries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UbloxSeries::Series7 => write!(f, "Series 7"),
            UbloxSeries::Series8 => write!(f, "Series 8"),
            UbloxSeries::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Full chip identity parsed from UBX-MON-VER response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UbloxChipInfo {
    pub sw_version: String,
    pub hw_version: String,
    pub extensions: Vec<String>,
    pub series: UbloxSeries,
    pub chip_name: String,
}

/// Parse a UBX-MON-VER response payload.
/// Layout: 30 bytes swVersion + 10 bytes hwVersion + N*30 extension strings
pub fn parse_mon_ver(payload: &[u8]) -> Option<UbloxChipInfo> {
    if payload.len() < 40 {
        return None;
    }

    let sw_version = String::from_utf8_lossy(&payload[0..30])
        .trim_end_matches('\0')
        .to_string();
    let hw_version = String::from_utf8_lossy(&payload[30..40])
        .trim_end_matches('\0')
        .to_string();

    let mut extensions = Vec::new();
    let mut offset = 40;
    while offset + 30 <= payload.len() {
        let ext = String::from_utf8_lossy(&payload[offset..offset + 30])
            .trim_end_matches('\0')
            .to_string();
        if !ext.is_empty() {
            extensions.push(ext);
        }
        offset += 30;
    }

    let (series, chip_name) = if hw_version.contains("G70") || hw_version.starts_with("00070") {
        (UbloxSeries::Series7, "u-blox 7".to_string())
    } else if hw_version.contains("M80")
        || hw_version.contains("M8030")
        || hw_version.starts_with("00080")
    {
        // Try to extract specific module name from extensions
        let name = extensions
            .iter()
            .find(|e| e.starts_with("MOD="))
            .map(|e| e.trim_start_matches("MOD=").to_string())
            .unwrap_or_else(|| {
                // Infer variant from firmware version string
                if let Some(fw) = extensions.iter().find(|e| e.starts_with("FWVER=")) {
                    let fw_str = fw.trim_start_matches("FWVER=");
                    if fw_str.starts_with("TIM") {
                        "NEO-M8T".to_string()
                    } else if fw_str.starts_with("HPG") {
                        "NEO-M8P".to_string()
                    } else if fw_str.starts_with("ADR") {
                        "NEO-M8U".to_string()
                    } else {
                        "u-blox M8".to_string()
                    }
                } else {
                    "u-blox M8".to_string()
                }
            });
        (UbloxSeries::Series8, name)
    } else {
        (
            UbloxSeries::Unknown,
            format!("u-blox (HW: {})", hw_version),
        )
    };

    Some(UbloxChipInfo {
        sw_version,
        hw_version,
        extensions,
        series,
        chip_name,
    })
}

// ============ UBX Message Construction ============

/// Calculate UBX checksum (Fletcher's algorithm over class+id+length+payload)
pub fn ubx_checksum(data: &[u8]) -> (u8, u8) {
    let mut ck_a: u8 = 0;
    let mut ck_b: u8 = 0;
    for &byte in data {
        ck_a = ck_a.wrapping_add(byte);
        ck_b = ck_b.wrapping_add(ck_a);
    }
    (ck_a, ck_b)
}

/// Build a complete UBX message with sync chars and checksum
pub fn build_ubx_message(class: u8, id: u8, payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u16;
    let mut msg = Vec::with_capacity(8 + payload.len());
    msg.push(UBX_SYNC_1);
    msg.push(UBX_SYNC_2);
    msg.push(class);
    msg.push(id);
    msg.push((len & 0xFF) as u8);
    msg.push((len >> 8) as u8);
    msg.extend_from_slice(payload);
    let (ck_a, ck_b) = ubx_checksum(&msg[2..]);
    msg.push(ck_a);
    msg.push(ck_b);
    msg
}

// ============ Chip Identification ============

/// Build UBX-MON-VER poll (empty payload = request)
pub fn build_mon_ver_poll() -> Vec<u8> {
    build_ubx_message(UBX_CLASS_MON, UBX_MON_VER, &[])
}

// ============ Constellation Configuration ============

/// Series 7 marine: GPS + SBAS only (Series 7 cannot do concurrent GNSS)
pub fn build_cfg_gnss_series7_marine() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(0x00); // msgVer
    payload.push(0x00); // numTrkChHw (read-only)
    payload.push(0xFF); // numTrkChUse: all available
    payload.push(0x02); // numConfigBlocks

    // GPS (gnssId=0): enable, 8 reserved, 16 max
    payload.extend_from_slice(&[0x00, 0x08, 0x10, 0x00, 0x01, 0x00, 0x01, 0x01]);
    // SBAS (gnssId=1): enable, 1 reserved, 3 max
    payload.extend_from_slice(&[0x01, 0x01, 0x03, 0x00, 0x01, 0x00, 0x01, 0x01]);

    build_ubx_message(UBX_CLASS_CFG, UBX_CFG_GNSS, &payload)
}

/// Series 8 marine: GPS + GLONASS + Galileo + SBAS (3 concurrent on M8, 72 channels)
pub fn build_cfg_gnss_series8_marine() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(0x00); // msgVer
    payload.push(0x00); // numTrkChHw (read-only)
    payload.push(0xFF); // numTrkChUse: all available
    payload.push(0x04); // numConfigBlocks

    // GPS (gnssId=0): enable, 8 reserved, 16 max
    payload.extend_from_slice(&[0x00, 0x08, 0x10, 0x00, 0x01, 0x00, 0x01, 0x01]);
    // SBAS (gnssId=1): enable, 1 reserved, 3 max
    payload.extend_from_slice(&[0x01, 0x01, 0x03, 0x00, 0x01, 0x00, 0x01, 0x01]);
    // Galileo (gnssId=2): enable, 4 reserved, 8 max
    payload.extend_from_slice(&[0x02, 0x04, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01]);
    // GLONASS (gnssId=6): enable, 8 reserved, 14 max
    payload.extend_from_slice(&[0x06, 0x08, 0x0E, 0x00, 0x01, 0x00, 0x01, 0x01]);

    build_ubx_message(UBX_CLASS_CFG, UBX_CFG_GNSS, &payload)
}

// ============ Navigation Configuration ============

/// UBX-CFG-NAV5: Dynamic model = Sea (5), fixMode = Auto 2D/3D (3)
/// Sea model: max alt 500m, max vel 25 m/s (~49 kn), zero vertical velocity
pub fn build_cfg_nav5_sea() -> Vec<u8> {
    #[rustfmt::skip]
    let payload: [u8; 36] = [
        0x01, 0x00,                         // mask: apply dynModel only
        0x05,                               // dynModel: Sea
        0x03,                               // fixMode: Auto 2D/3D
        0x00, 0x00, 0x00, 0x00,             // fixedAlt (not used)
        0x10, 0x27, 0x00, 0x00,             // fixedAltVar: 10000 (1.0 m^2)
        0x05,                               // minElev: 5 degrees
        0x00,                               // drLimit (reserved)
        0xFA, 0x00,                         // pDop: 250 (25.0)
        0xFA, 0x00,                         // tDop: 250 (25.0)
        0x64, 0x00,                         // pAcc: 100m
        0x2C, 0x01,                         // tAcc: 300m
        0x00,                               // staticHoldThresh: 0 (marine = moving)
        0x00,                               // dgnssTimeout
        0x00, 0x00, 0x00, 0x00,             // cnoThreshNumSVs, cnoThresh, reserved
        0x00, 0x00,                         // staticHoldMaxDist
        0x00,                               // utcStandard: auto
        0x00, 0x00, 0x00, 0x00, 0x00,       // reserved
    ];
    build_ubx_message(UBX_CLASS_CFG, UBX_CFG_NAV5, &payload)
}

/// UBX-CFG-RATE: 1Hz measurement rate (1000ms), GPS time reference
pub fn build_cfg_rate_1hz() -> Vec<u8> {
    #[rustfmt::skip]
    let payload: [u8; 6] = [
        0xE8, 0x03,     // measRate: 1000ms
        0x01, 0x00,     // navRate: 1 cycle
        0x01, 0x00,     // timeRef: GPS time
    ];
    build_ubx_message(UBX_CLASS_CFG, UBX_CFG_RATE, &payload)
}

// ============ SBAS Configuration ============

/// UBX-CFG-SBAS: Enable SBAS with ranging, diff corrections, integrity; auto-scan all PRNs
pub fn build_cfg_sbas_enable() -> Vec<u8> {
    #[rustfmt::skip]
    let payload: [u8; 8] = [
        0x01,                       // mode: enabled
        0x07,                       // usage: range + diffCorr + integrity
        0x03,                       // maxSBAS: 3
        0x00,                       // scanmode2
        0x00, 0x00, 0x00, 0x00,    // scanmode1: 0 = auto-scan all
    ];
    build_ubx_message(UBX_CLASS_CFG, UBX_CFG_SBAS, &payload)
}

// ============ NMEA Message Configuration ============

/// Build UBX-CFG-MSG for a specific NMEA sentence (8-byte form)
fn build_cfg_msg(nmea_msg_id: u8, rate: u8) -> Vec<u8> {
    // 8-byte form: class, id, rate for I2C, UART1, UART2, USB, SPI, reserved
    let payload = [0xF0, nmea_msg_id, 0x00, rate, 0x00, rate, 0x00, 0x00];
    build_ubx_message(UBX_CLASS_CFG, UBX_CFG_MSG, &payload)
}

/// All NMEA message config commands: enable GGA, RMC, VTG, GSA, GSV; disable GLL
pub fn build_nmea_message_config() -> Vec<Vec<u8>> {
    vec![
        build_cfg_msg(NMEA_GGA, 1), // Position fix
        build_cfg_msg(NMEA_RMC, 1), // Recommended minimum
        build_cfg_msg(NMEA_VTG, 1), // Course over ground
        build_cfg_msg(NMEA_GSA, 1), // DOP and active sats
        build_cfg_msg(NMEA_GSV, 1), // Satellites in view
        build_cfg_msg(NMEA_GLL, 0), // Disable (redundant with GGA)
    ]
}

/// UBX-CFG-NMEA: Extended talker IDs for multi-constellation
pub fn build_cfg_nmea_extended() -> Vec<u8> {
    let payload = [
        0x00, 0x23, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    ];
    build_ubx_message(UBX_CLASS_CFG, UBX_CFG_NMEA, &payload)
}

// ============ Save Configuration ============

/// UBX-CFG-CFG: Save current config to all non-volatile memory (BBR + Flash + EEPROM + SPI)
pub fn build_cfg_save_all() -> Vec<u8> {
    #[rustfmt::skip]
    let payload: [u8; 13] = [
        0x00, 0x00, 0x00, 0x00,     // clearMask: don't clear
        0x1F, 0x1F, 0x00, 0x00,     // saveMask: all sections
        0x00, 0x00, 0x00, 0x00,     // loadMask: don't load
        0x17,                        // deviceMask: BBR + Flash + EEPROM + SPI
    ];
    build_ubx_message(UBX_CLASS_CFG, UBX_CFG_CFG, &payload)
}

// ============ Full Optimization Sequence ============

/// Get the complete ordered list of UBX commands for a marine optimization profile.
/// The save command is always last.
pub fn get_optimization_commands(series: &UbloxSeries) -> Vec<Vec<u8>> {
    let mut commands = Vec::new();

    // 1. Constellation config (series-specific)
    match series {
        UbloxSeries::Series7 => commands.push(build_cfg_gnss_series7_marine()),
        UbloxSeries::Series8 | UbloxSeries::Unknown => {
            commands.push(build_cfg_gnss_series8_marine());
        }
    }

    // 2. Dynamic model: Sea
    commands.push(build_cfg_nav5_sea());

    // 3. Measurement rate: 1Hz
    commands.push(build_cfg_rate_1hz());

    // 4. SBAS enabled with full corrections
    commands.push(build_cfg_sbas_enable());

    // 5. Extended NMEA talker IDs
    commands.push(build_cfg_nmea_extended());

    // 6. Enable/disable individual NMEA sentences
    commands.extend(build_nmea_message_config());

    // 7. Save to flash (always last)
    commands.push(build_cfg_save_all());

    commands
}

/// Get a human-readable profile name for a series
pub fn profile_name(series: &UbloxSeries) -> &'static str {
    match series {
        UbloxSeries::Series7 => "Series 7 Marine (GPS + SBAS)",
        UbloxSeries::Series8 => "Series 8 Marine (GPS + GLONASS + Galileo + SBAS)",
        UbloxSeries::Unknown => "Generic Marine",
    }
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ubx_checksum() {
        // Known: MON-VER poll = B5 62 0A 04 00 00 -> checksum over [0A 04 00 00]
        let data = [0x0A, 0x04, 0x00, 0x00];
        let (ck_a, ck_b) = ubx_checksum(&data);
        assert_eq!(ck_a, 0x0E);
        assert_eq!(ck_b, 0x34);
    }

    #[test]
    fn test_build_ubx_message_mon_ver_poll() {
        let msg = build_mon_ver_poll();
        assert_eq!(msg.len(), 8); // sync(2) + class(1) + id(1) + len(2) + ck(2)
        assert_eq!(msg[0], 0xB5);
        assert_eq!(msg[1], 0x62);
        assert_eq!(msg[2], 0x0A); // class: MON
        assert_eq!(msg[3], 0x04); // id: VER
        assert_eq!(msg[4], 0x00); // len low
        assert_eq!(msg[5], 0x00); // len high
        assert_eq!(msg[6], 0x0E); // ck_a
        assert_eq!(msg[7], 0x34); // ck_b
    }

    #[test]
    fn test_parse_mon_ver_series8() {
        // Simulate a MON-VER response for a NEO-M8N
        let mut payload = Vec::new();
        // SW version (30 bytes)
        let sw = b"ROM CORE 3.01 (107888)\0\0\0\0\0\0\0\0";
        payload.extend_from_slice(sw);
        // HW version (10 bytes)
        let hw = b"00080000\0\0";
        payload.extend_from_slice(hw);
        // Extension 1 (30 bytes)
        let ext1 = b"FWVER=SPG 3.01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        payload.extend_from_slice(ext1);
        // Extension 2 (30 bytes)
        let ext2 = b"PROTVER=18.00\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        payload.extend_from_slice(ext2);

        let info = parse_mon_ver(&payload).unwrap();
        assert_eq!(info.series, UbloxSeries::Series8);
        assert_eq!(info.chip_name, "u-blox M8");
        assert!(info.sw_version.contains("ROM CORE 3.01"));
        assert_eq!(info.extensions.len(), 2);
    }

    #[test]
    fn test_parse_mon_ver_series7() {
        let mut payload = Vec::new();
        let sw = b"1.00 (59842)\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        payload.extend_from_slice(sw);
        let hw = b"00070000\0\0";
        payload.extend_from_slice(hw);

        let info = parse_mon_ver(&payload).unwrap();
        assert_eq!(info.series, UbloxSeries::Series7);
        assert_eq!(info.chip_name, "u-blox 7");
    }

    #[test]
    fn test_parse_mon_ver_too_short() {
        assert!(parse_mon_ver(&[0u8; 30]).is_none());
    }

    #[test]
    fn test_optimization_commands_series7() {
        let cmds = get_optimization_commands(&UbloxSeries::Series7);
        // Should not contain Galileo or GLONASS constellation blocks
        // First command is CFG-GNSS with 2 config blocks (GPS + SBAS)
        assert!(cmds.len() >= 10); // gnss + nav5 + rate + sbas + nmea_ext + 6 msg configs + save
        // First command: CFG-GNSS, check numConfigBlocks = 2
        let gnss_cmd = &cmds[0];
        assert_eq!(gnss_cmd[2], 0x06); // class CFG
        assert_eq!(gnss_cmd[3], 0x3E); // id GNSS
        // Payload byte 3 (offset 9 in message) = numConfigBlocks
        assert_eq!(gnss_cmd[9], 0x02);
    }

    #[test]
    fn test_optimization_commands_series8() {
        let cmds = get_optimization_commands(&UbloxSeries::Series8);
        let gnss_cmd = &cmds[0];
        assert_eq!(gnss_cmd[2], 0x06);
        assert_eq!(gnss_cmd[3], 0x3E);
        // numConfigBlocks = 4 (GPS + SBAS + Galileo + GLONASS)
        assert_eq!(gnss_cmd[9], 0x04);
    }

    #[test]
    fn test_cfg_nav5_sea_dynmodel() {
        let msg = build_cfg_nav5_sea();
        assert_eq!(msg[2], 0x06); // class CFG
        assert_eq!(msg[3], 0x24); // id NAV5
        // Payload byte 2 (offset 8 in message) = dynModel
        assert_eq!(msg[8], 0x05); // Sea
    }

    #[test]
    fn test_cfg_rate_1hz() {
        let msg = build_cfg_rate_1hz();
        assert_eq!(msg[2], 0x06);
        assert_eq!(msg[3], 0x08);
        // measRate = 1000ms = 0x03E8
        assert_eq!(msg[6], 0xE8);
        assert_eq!(msg[7], 0x03);
    }

    #[test]
    fn test_cfg_save_all() {
        let msg = build_cfg_save_all();
        assert_eq!(msg[2], 0x06);
        assert_eq!(msg[3], 0x09);
        // deviceMask at payload offset 12 (message offset 18)
        assert_eq!(msg[18], 0x17);
    }

    #[test]
    fn test_last_command_is_save() {
        let cmds = get_optimization_commands(&UbloxSeries::Series8);
        let last = cmds.last().unwrap();
        assert_eq!(last[2], 0x06); // CFG
        assert_eq!(last[3], 0x09); // CFG-CFG (save)
    }
}
