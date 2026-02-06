// GPS hardware detection and serial reading module
// Simplified from VortexNav: single-source, auto-detect, no failover/TCP/simulated

use crate::nmea::{GpsData, NmeaParser};
use serde::{Deserialize, Serialize};
use serialport::SerialPortType;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use thiserror::Error;

// ============ UBX Protocol Support for u-blox Configuration ============

/// Calculate UBX checksum (Fletcher's algorithm)
fn ubx_checksum(data: &[u8]) -> (u8, u8) {
    let mut ck_a: u8 = 0;
    let mut ck_b: u8 = 0;
    for byte in data {
        ck_a = ck_a.wrapping_add(*byte);
        ck_b = ck_b.wrapping_add(ck_a);
    }
    (ck_a, ck_b)
}

/// Build a complete UBX message with sync chars and checksum
fn build_ubx_message(class: u8, id: u8, payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u16;
    let mut msg = Vec::with_capacity(8 + payload.len());
    msg.push(0xB5);
    msg.push(0x62);
    msg.push(class);
    msg.push(id);
    msg.push((len & 0xFF) as u8);
    msg.push((len >> 8) as u8);
    msg.extend_from_slice(payload);
    let checksum_data = &msg[2..];
    let (ck_a, ck_b) = ubx_checksum(checksum_data);
    msg.push(ck_a);
    msg.push(ck_b);
    msg
}

/// Build UBX-CFG-GNSS message to enable GPS + GLONASS + SBAS
fn build_ubx_cfg_gnss_multi_constellation() -> Vec<u8> {
    let mut payload = Vec::new();
    // Header
    payload.push(0x00); // msgVer
    payload.push(0x00); // numTrkChHw
    payload.push(0xFF); // numTrkChUse: all available
    payload.push(0x03); // numConfigBlocks: GPS + SBAS + GLONASS

    // GPS (gnssId = 0)
    payload.extend_from_slice(&[0x00, 0x04, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00]);
    // SBAS (gnssId = 1)
    payload.extend_from_slice(&[0x01, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00]);
    // GLONASS (gnssId = 6)
    payload.extend_from_slice(&[0x06, 0x04, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00]);

    build_ubx_message(0x06, 0x3E, &payload)
}

/// Build UBX-CFG-MSG to enable GLONASS GSV sentences
fn build_ubx_cfg_msg_glgsv_enable() -> Vec<u8> {
    let payload = vec![0xF0, 0x03, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00];
    build_ubx_message(0x06, 0x01, &payload)
}

/// Build UBX-CFG-NMEA for extended talker IDs
fn build_ubx_cfg_nmea_extended() -> Vec<u8> {
    let payload = vec![0x00, 0x23, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
    build_ubx_message(0x06, 0x17, &payload)
}

/// Configure a u-blox GPS receiver for multi-constellation operation
fn configure_ublox_multi_constellation(port: &mut Box<dyn serialport::SerialPort>) -> Result<(), std::io::Error> {
    use std::io::Read;

    log::info!("Configuring GPS receiver for multi-constellation (GPS + GLONASS)...");
    thread::sleep(Duration::from_millis(100));

    // Enable GPS + GLONASS constellations
    let gnss_cmd = build_ubx_cfg_gnss_multi_constellation();
    port.write_all(&gnss_cmd)?;
    port.flush()?;
    thread::sleep(Duration::from_millis(250));

    // Enable extended NMEA with proper talker IDs
    let nmea_cmd = build_ubx_cfg_nmea_extended();
    port.write_all(&nmea_cmd)?;
    port.flush()?;
    thread::sleep(Duration::from_millis(250));

    // Enable GLONASS GSV sentences
    let glgsv_cmd = build_ubx_cfg_msg_glgsv_enable();
    port.write_all(&glgsv_cmd)?;
    port.flush()?;
    thread::sleep(Duration::from_millis(250));

    // Drain any binary UBX response data
    let mut drain_buf = [0u8; 512];
    let original_timeout = port.timeout();
    port.set_timeout(Duration::from_millis(100))?;

    loop {
        match port.read(&mut drain_buf) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }

    port.set_timeout(original_timeout)?;
    log::info!("GPS multi-constellation configuration complete");
    Ok(())
}

// ============ GPS Types ============

#[derive(Error, Debug)]
pub enum GpsError {
    #[error("Serial port error: {0}")]
    SerialPort(#[from] serialport::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No GPS device detected")]
    NoGpsDetected,
}

/// Information about a detected serial port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPort {
    pub port_name: String,
    pub port_type: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub is_likely_gps: bool,
}

/// GPS connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GpsConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    ReceivingData,
    Error,
}

/// Current GPS source status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsSourceStatus {
    pub port_name: Option<String>,
    pub status: GpsConnectionStatus,
    pub last_error: Option<String>,
    pub sentences_received: u64,
    pub last_fix_time: Option<String>,
}

impl Default for GpsSourceStatus {
    fn default() -> Self {
        Self {
            port_name: None,
            status: GpsConnectionStatus::Disconnected,
            last_error: None,
            sentences_received: 0,
            last_fix_time: None,
        }
    }
}

// NMEA sentence buffer size
const NMEA_BUFFER_SIZE: usize = 100;

// ============ GPS Manager ============

pub struct GpsManager {
    pub data: Arc<RwLock<GpsData>>,
    pub status: Arc<RwLock<GpsSourceStatus>>,
    stop_flag: Arc<AtomicBool>,
    reader_handle: std::sync::Mutex<Option<thread::JoinHandle<()>>>,
    nmea_buffer: Arc<RwLock<Vec<String>>>,
}

impl GpsManager {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(GpsData::default())),
            status: Arc::new(RwLock::new(GpsSourceStatus::default())),
            stop_flag: Arc::new(AtomicBool::new(false)),
            reader_handle: std::sync::Mutex::new(None),
            nmea_buffer: Arc::new(RwLock::new(Vec::with_capacity(NMEA_BUFFER_SIZE))),
        }
    }

    /// Get recent NMEA sentences
    pub fn get_nmea_buffer(&self) -> Vec<String> {
        self.nmea_buffer.read().unwrap().clone()
    }

    /// Clear the NMEA buffer
    pub fn clear_nmea_buffer(&self) {
        self.nmea_buffer.write().unwrap().clear();
    }

    /// Enumerate all available serial ports
    pub fn list_serial_ports() -> Result<Vec<DetectedPort>, GpsError> {
        let ports = serialport::available_ports()?;

        let detected: Vec<DetectedPort> = ports
            .into_iter()
            .map(|port| {
                let (port_type, manufacturer, product, serial_number, is_likely_gps) =
                    match &port.port_type {
                        SerialPortType::UsbPort(info) => {
                            let mfr = info.manufacturer.clone();
                            let prod = info.product.clone();
                            let likely_gps = is_likely_gps_device(&mfr, &prod);
                            (
                                "USB".to_string(),
                                mfr,
                                prod,
                                info.serial_number.clone(),
                                likely_gps,
                            )
                        }
                        SerialPortType::BluetoothPort => {
                            ("Bluetooth".to_string(), None, None, None, false)
                        }
                        SerialPortType::PciPort => ("PCI".to_string(), None, None, None, false),
                        SerialPortType::Unknown => {
                            ("Unknown".to_string(), None, None, None, false)
                        }
                    };

                DetectedPort {
                    port_name: port.port_name,
                    port_type,
                    manufacturer,
                    product,
                    serial_number,
                    is_likely_gps,
                }
            })
            .collect();

        Ok(detected)
    }

    /// Test if a port is a GPS device by reading a few sentences
    pub fn test_port(port_name: &str, baud_rate: u32, timeout_ms: u64) -> Result<bool, GpsError> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(timeout_ms))
            .open()?;

        let mut reader = BufReader::new(port);
        let mut line = String::new();
        let mut nmea_count = 0;

        for _ in 0..10 {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with('$')
                        && (trimmed.contains("GP")
                            || trimmed.contains("GN")
                            || trimmed.contains("GL"))
                    {
                        nmea_count += 1;
                        if nmea_count >= 2 {
                            return Ok(true);
                        }
                    }
                }
                Err(_) => break,
            }
        }

        Ok(nmea_count > 0)
    }

    /// Auto-detect GPS hardware: scan all ports, test likely candidates first
    pub fn auto_detect_gps() -> Result<(DetectedPort, u32), GpsError> {
        let ports = Self::list_serial_ports()?;

        // Sort: likely GPS devices first
        let mut sorted = ports;
        sorted.sort_by_key(|p| if p.is_likely_gps { 0 } else { 1 });

        let baud_rates = [4800u32, 9600, 115200];

        for port in &sorted {
            for &baud in &baud_rates {
                log::info!("Testing {} at {} baud...", port.port_name, baud);
                match Self::test_port(&port.port_name, baud, 3000) {
                    Ok(true) => {
                        log::info!("GPS detected on {} at {} baud", port.port_name, baud);
                        return Ok((port.clone(), baud));
                    }
                    Ok(false) => continue,
                    Err(e) => {
                        log::debug!("Port test failed for {}: {}", port.port_name, e);
                        continue;
                    }
                }
            }
        }

        Err(GpsError::NoGpsDetected)
    }

    /// Get current GPS data
    pub fn get_data(&self) -> GpsData {
        self.data.read().unwrap().clone()
    }

    /// Get current status
    pub fn get_status(&self) -> GpsSourceStatus {
        self.status.read().unwrap().clone()
    }

    /// Connect to a specific GPS port and start reading
    pub fn connect(&self, port_name: &str, baud_rate: u32) -> Result<(), GpsError> {
        // Stop any existing reader
        self.disconnect();

        // Reset stop flag
        self.stop_flag.store(false, Ordering::SeqCst);

        // Update status to connecting
        {
            let mut status = self.status.write().unwrap();
            status.port_name = Some(port_name.to_string());
            status.status = GpsConnectionStatus::Connecting;
            status.last_error = None;
            status.sentences_received = 0;
        }

        // Clear previous data
        {
            let mut data = self.data.write().unwrap();
            *data = GpsData::default();
        }

        let stop_flag = Arc::clone(&self.stop_flag);
        let data_lock = Arc::clone(&self.data);
        let status_lock = Arc::clone(&self.status);
        let nmea_buffer_lock = Arc::clone(&self.nmea_buffer);
        let port_name_owned = port_name.to_string();

        let handle = thread::spawn(move || {
            if let Err(e) = Self::read_from_serial(
                &stop_flag,
                &data_lock,
                &status_lock,
                &nmea_buffer_lock,
                &port_name_owned,
                baud_rate,
            ) {
                log::error!("GPS reader error: {}", e);
                let mut status = status_lock.write().unwrap();
                status.last_error = Some(e.to_string());
                status.status = GpsConnectionStatus::Error;
            }
        });

        *self.reader_handle.lock().unwrap() = Some(handle);
        Ok(())
    }

    /// Stop GPS reading
    pub fn disconnect(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);

        if let Some(handle) = self.reader_handle.lock().unwrap().take() {
            thread::sleep(Duration::from_millis(100));
            drop(handle);
        }

        let mut status = self.status.write().unwrap();
        status.status = GpsConnectionStatus::Disconnected;
    }

    /// Read GPS data from a serial port
    fn read_from_serial(
        stop_flag: &Arc<AtomicBool>,
        data_lock: &RwLock<GpsData>,
        status_lock: &RwLock<GpsSourceStatus>,
        nmea_buffer_lock: &RwLock<Vec<String>>,
        port_name: &str,
        baud_rate: u32,
    ) -> Result<(), GpsError> {
        let mut port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(1000))
            .open()?;

        // Update status to connected
        {
            let mut status = status_lock.write().unwrap();
            status.status = GpsConnectionStatus::Connected;
            status.last_error = None;
        }

        // Only configure via UBX if this looks like a u-blox receiver
        if is_ublox_device(port_name) {
            log::info!("u-blox device detected, sending UBX configuration...");
            if let Err(e) = configure_ublox_multi_constellation(&mut port) {
                log::warn!("Failed to configure multi-constellation (non-fatal): {}", e);
            }
        } else {
            log::info!("Non-u-blox device, skipping UBX configuration");
        }

        let parser = NmeaParser::new();
        let mut reader = BufReader::new(port);
        let mut line = String::new();
        let mut sentences_received: u64 = 0;

        while !stop_flag.load(Ordering::SeqCst) {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with('$') {
                        sentences_received += 1;

                        // Add to NMEA buffer (ring buffer)
                        {
                            let mut buffer = nmea_buffer_lock.write().unwrap();
                            if buffer.len() >= NMEA_BUFFER_SIZE {
                                buffer.remove(0);
                            }
                            buffer.push(trimmed.to_string());
                        }

                        // Parse the NMEA sentence
                        if let Ok(new_data) = parser.parse_sentence(trimmed) {
                            let mut data = data_lock.write().unwrap();
                            if new_data.latitude.is_some() { data.latitude = new_data.latitude; }
                            if new_data.longitude.is_some() { data.longitude = new_data.longitude; }
                            if new_data.speed_knots.is_some() { data.speed_knots = new_data.speed_knots; }
                            if new_data.course.is_some() { data.course = new_data.course; }
                            if new_data.heading.is_some() { data.heading = new_data.heading; }
                            if new_data.altitude.is_some() { data.altitude = new_data.altitude; }
                            if new_data.fix_quality.is_some() { data.fix_quality = new_data.fix_quality; }
                            if new_data.satellites.is_some() { data.satellites = new_data.satellites; }
                            if new_data.hdop.is_some() { data.hdop = new_data.hdop; }
                            if new_data.vdop.is_some() { data.vdop = new_data.vdop; }
                            if new_data.pdop.is_some() { data.pdop = new_data.pdop; }
                            if new_data.timestamp.is_some() { data.timestamp = new_data.timestamp.clone(); }
                            if new_data.fix_type.is_some() { data.fix_type = new_data.fix_type.clone(); }
                            if !new_data.satellites_info.is_empty() { data.satellites_info = new_data.satellites_info.clone(); }
                        }

                        // Update status
                        {
                            let mut status = status_lock.write().unwrap();
                            status.status = GpsConnectionStatus::ReceivingData;
                            status.sentences_received = sentences_received;
                            if let Some(ref ts) = data_lock.read().unwrap().timestamp {
                                status.last_fix_time = Some(ts.clone());
                            }
                        }
                    }
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::TimedOut {
                        return Err(GpsError::Io(e));
                    }
                }
            }
        }

        Ok(())
    }
}

impl Drop for GpsManager {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Heuristic to detect if a USB device is likely a GPS
fn is_likely_gps_device(manufacturer: &Option<String>, product: &Option<String>) -> bool {
    let keywords = [
        "gps", "gnss", "u-blox", "ublox", "sirf", "nmea", "garmin", "globalsat",
        "bu-353", "vk-162", "g-mouse", "receiver", "navigation",
    ];

    let check_string = |s: &Option<String>| -> bool {
        if let Some(ref text) = s {
            let lower = text.to_lowercase();
            keywords.iter().any(|kw| lower.contains(kw))
        } else {
            false
        }
    };

    check_string(manufacturer) || check_string(product)
}

/// Check if a connected device is a u-blox receiver (safe to send UBX commands)
fn is_ublox_device(port_name: &str) -> bool {
    if let Ok(ports) = serialport::available_ports() {
        for port in &ports {
            if port.port_name == port_name {
                if let SerialPortType::UsbPort(info) = &port.port_type {
                    // u-blox USB vendor ID is 0x1546
                    if info.vid == 0x1546 {
                        return true;
                    }
                    // Also check manufacturer/product strings
                    let check = |s: &Option<String>| -> bool {
                        s.as_ref()
                            .map(|t| {
                                let lower = t.to_lowercase();
                                lower.contains("u-blox") || lower.contains("ublox")
                            })
                            .unwrap_or(false)
                    };
                    if check(&info.manufacturer) || check(&info.product) {
                        return true;
                    }
                }
            }
        }
    }
    false
}
