// NMEA 0183 parser module for GPS data

use nmea::Nmea;
use nmea::sentences::{FixType, GnssType};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum NmeaError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Serial port error: {0}")]
    SerialPort(String),
    #[error("No GPS fix")]
    NoFix,
}

// Individual satellite information from GSV sentences
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SatelliteInfo {
    pub prn: u32,           // Satellite PRN number
    pub elevation: Option<f32>,  // Elevation in degrees (0-90)
    pub azimuth: Option<f32>,    // Azimuth in degrees (0-359)
    pub snr: Option<f32>,        // Signal-to-noise ratio (0-99 dB)
    pub constellation: String,   // GPS, GLONASS, Galileo, etc.
}

// GPS position data sent to frontend
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpsData {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub speed_knots: Option<f64>,     // SOG - Speed Over Ground
    pub course: Option<f64>,           // COG - Course Over Ground
    pub heading: Option<f64>,          // HDG - True heading (from compass)
    pub altitude: Option<f64>,
    pub fix_quality: Option<u8>,
    pub satellites: Option<u32>,
    pub hdop: Option<f32>,             // Horizontal dilution of precision
    pub vdop: Option<f32>,             // Vertical dilution of precision
    pub pdop: Option<f32>,             // Position dilution of precision
    pub timestamp: Option<String>,
    pub fix_type: Option<String>,      // No fix, 2D, 3D
    pub satellites_info: Vec<SatelliteInfo>,  // Individual satellite data
}

// NMEA parser state
pub struct NmeaParser {
    nmea: Mutex<Nmea>,
}

#[allow(dead_code)]
impl NmeaParser {
    pub fn new() -> Self {
        Self {
            nmea: Mutex::new(Nmea::default()),
        }
    }

    /// Parse an NMEA sentence and return updated GPS data
    pub fn parse_sentence(&self, sentence: &str) -> Result<GpsData, NmeaError> {
        let mut nmea = self.nmea.lock().unwrap();

        // Parse the sentence
        nmea.parse(sentence).map_err(|e| NmeaError::Parse(format!("{:?}", e)))?;

        // Extract satellite information
        let satellites_info: Vec<SatelliteInfo> = nmea.satellites()
            .iter()
            .map(|sat| {
                let constellation = match sat.gnss_type() {
                    GnssType::Galileo => "Galileo",
                    GnssType::Gps => "GPS",
                    GnssType::Glonass => "GLONASS",
                    GnssType::Beidou => "BeiDou",
                    GnssType::Qzss => "QZSS",
                    GnssType::NavIC => "NavIC",
                }.to_string();

                SatelliteInfo {
                    prn: sat.prn(),
                    elevation: sat.elevation(),
                    azimuth: sat.azimuth(),
                    snr: sat.snr(),
                    constellation,
                }
            })
            .collect();

        // Determine fix type string
        let fix_type = nmea.fix_type.map(|f| match f {
            FixType::Invalid => "No Fix".to_string(),
            FixType::Gps => "GPS".to_string(),
            FixType::DGps => "DGPS".to_string(),
            FixType::Pps => "PPS".to_string(),
            FixType::Rtk => "RTK".to_string(),
            FixType::FloatRtk => "Float RTK".to_string(),
            FixType::Estimated => "Estimated".to_string(),
            FixType::Manual => "Manual".to_string(),
            FixType::Simulation => "Simulation".to_string(),
        });

        // Extract all available data (convert f32 to f64 where needed)
        let data = GpsData {
            latitude: nmea.latitude,
            longitude: nmea.longitude,
            speed_knots: nmea.speed_over_ground.map(|v| v as f64),
            course: nmea.true_course.map(|v| v as f64),
            heading: None, // Would come from HDT/HDG sentence
            altitude: nmea.altitude.map(|v| v as f64),
            fix_quality: nmea.fix_type.map(|f| f as u8),
            satellites: nmea.num_of_fix_satellites,
            hdop: nmea.hdop,
            vdop: nmea.vdop,
            pdop: nmea.pdop,
            timestamp: nmea.fix_time.map(|t| t.to_string()),
            fix_type,
            satellites_info,
        };

        Ok(data)
    }

    /// Parse multiple lines of NMEA data
    pub fn parse_batch(&self, data: &str) -> GpsData {
        let mut latest = GpsData::default();

        for line in data.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                if let Ok(gps) = self.parse_sentence(trimmed) {
                    // Merge non-None values
                    if gps.latitude.is_some() { latest.latitude = gps.latitude; }
                    if gps.longitude.is_some() { latest.longitude = gps.longitude; }
                    if gps.speed_knots.is_some() { latest.speed_knots = gps.speed_knots; }
                    if gps.course.is_some() { latest.course = gps.course; }
                    if gps.heading.is_some() { latest.heading = gps.heading; }
                    if gps.altitude.is_some() { latest.altitude = gps.altitude; }
                    if gps.fix_quality.is_some() { latest.fix_quality = gps.fix_quality; }
                    if gps.satellites.is_some() { latest.satellites = gps.satellites; }
                    if gps.hdop.is_some() { latest.hdop = gps.hdop; }
                    if gps.vdop.is_some() { latest.vdop = gps.vdop; }
                    if gps.pdop.is_some() { latest.pdop = gps.pdop; }
                    if gps.timestamp.is_some() { latest.timestamp = gps.timestamp; }
                    if gps.fix_type.is_some() { latest.fix_type = gps.fix_type; }
                    if !gps.satellites_info.is_empty() { latest.satellites_info = gps.satellites_info; }
                }
            }
        }

        latest
    }

    /// Reset parser state
    pub fn reset(&self) {
        let mut nmea = self.nmea.lock().unwrap();
        *nmea = Nmea::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gga() {
        let parser = NmeaParser::new();
        let sentence = "$GPGGA,092750.000,5321.6802,N,00630.3372,W,1,8,1.03,61.7,M,55.2,M,,*76";
        let result = parser.parse_sentence(sentence);
        assert!(result.is_ok(), "Failed to parse GGA: {:?}", result.err());
        let gps = result.unwrap();
        assert!(gps.latitude.is_some(), "Latitude should be parsed");
        assert!(gps.longitude.is_some(), "Longitude should be parsed");
        let lat = gps.latitude.unwrap();
        let lon = gps.longitude.unwrap();
        assert!((lat - 53.36).abs() < 0.1, "Latitude should be ~53.36, got {}", lat);
        assert!((lon - (-6.50)).abs() < 0.1, "Longitude should be ~-6.50, got {}", lon);
    }

    #[test]
    fn test_parse_rmc() {
        let parser = NmeaParser::new();
        let sentence = "$GPRMC,225446,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E*68";
        let result = parser.parse_sentence(sentence);
        assert!(result.is_ok(), "Failed to parse RMC: {:?}", result.err());
        let gps = result.unwrap();
        assert!(gps.speed_knots.is_some(), "Speed should be parsed");
        assert!(gps.course.is_some(), "Course should be parsed");
    }
}
