// GPS test criteria engine - configurable pass/fail thresholds

use crate::nmea::GpsData;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;

/// Configurable test criteria with sensible defaults for u-blox NEO-M8N
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCriteria {
    pub min_satellites: u32,
    pub max_hdop: f32,
    pub max_pdop: f32,
    pub min_avg_snr: f32,
    pub min_strong_satellites: u32,
    pub max_ttff_seconds: u64,
    pub min_constellations: u32,
    pub min_fix_quality: u8,
    pub stability_duration_seconds: u64,
}

impl Default for TestCriteria {
    fn default() -> Self {
        Self {
            min_satellites: 6,
            max_hdop: 2.0,
            max_pdop: 3.0,
            min_avg_snr: 25.0,
            min_strong_satellites: 4,
            max_ttff_seconds: 60,
            min_constellations: 2,
            min_fix_quality: 1,
            stability_duration_seconds: 10,
        }
    }
}

/// Result of evaluating a single criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResult {
    pub name: String,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
}

/// Overall test verdict
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TestVerdict {
    Pass,
    Fail,
    Running,
    NotStarted,
    TimedOut,
}

/// Device hardware identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub port_name: String,
    pub port_type: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}

/// Complete test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub verdict: TestVerdict,
    pub criteria_results: Vec<CriterionResult>,
    pub ttff_seconds: Option<f64>,
    pub test_duration_seconds: f64,
    pub device_info: DeviceInfo,
    pub timestamp: String,
    pub best_gps_data: Option<GpsData>,
}

/// Test state machine
pub struct TestRunner {
    pub criteria: TestCriteria,
    start_time: Option<Instant>,
    first_fix_time: Option<Instant>,
    stable_since: Option<Instant>,
    pub verdict: TestVerdict,
    pub device_info: DeviceInfo,
    last_criteria_results: Vec<CriterionResult>,
    best_satellites: u32,
}

impl TestRunner {
    pub fn new(criteria: TestCriteria, device_info: DeviceInfo) -> Self {
        Self {
            criteria,
            start_time: None,
            first_fix_time: None,
            stable_since: None,
            verdict: TestVerdict::NotStarted,
            device_info,
            last_criteria_results: Vec::new(),
            best_satellites: 0,
        }
    }

    /// Start the test
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.first_fix_time = None;
        self.stable_since = None;
        self.verdict = TestVerdict::Running;
        self.last_criteria_results.clear();
        self.best_satellites = 0;
    }

    /// Get elapsed seconds since test start
    pub fn elapsed_seconds(&self) -> f64 {
        self.start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Get TTFF in seconds (None if no fix yet)
    pub fn ttff_seconds(&self) -> Option<f64> {
        match (self.start_time, self.first_fix_time) {
            (Some(start), Some(fix)) => Some(fix.duration_since(start).as_secs_f64()),
            _ => None,
        }
    }

    /// Evaluate GPS data against criteria, advancing the state machine
    pub fn evaluate(&mut self, data: &GpsData) -> Vec<CriterionResult> {
        if self.verdict != TestVerdict::Running {
            return self.last_criteria_results.clone();
        }

        // Check TTFF timeout
        let elapsed = self.elapsed_seconds();
        let has_fix = data.fix_quality.unwrap_or(0) >= self.criteria.min_fix_quality;

        // Record first fix
        if has_fix && self.first_fix_time.is_none() {
            self.first_fix_time = Some(Instant::now());
            log::info!("First fix acquired at {:.1}s", elapsed);
        }

        // Track best satellite count
        let sat_count = data.satellites.unwrap_or(0);
        if sat_count > self.best_satellites {
            self.best_satellites = sat_count;
        }

        // Evaluate all criteria
        let mut results = Vec::new();

        // 1. Satellite count
        results.push(CriterionResult {
            name: "Satellite Count".into(),
            passed: sat_count >= self.criteria.min_satellites,
            expected: format!(">= {}", self.criteria.min_satellites),
            actual: format!("{}", sat_count),
        });

        // 2. HDOP
        let hdop_pass = data.hdop.map_or(false, |h| h <= self.criteria.max_hdop);
        results.push(CriterionResult {
            name: "HDOP".into(),
            passed: hdop_pass,
            expected: format!("<= {:.1}", self.criteria.max_hdop),
            actual: data.hdop.map_or("-".into(), |h| format!("{:.1}", h)),
        });

        // 3. PDOP
        let pdop_pass = data.pdop.map_or(false, |p| p <= self.criteria.max_pdop);
        results.push(CriterionResult {
            name: "PDOP".into(),
            passed: pdop_pass,
            expected: format!("<= {:.1}", self.criteria.max_pdop),
            actual: data.pdop.map_or("-".into(), |p| format!("{:.1}", p)),
        });

        // 4. Average SNR
        let avg_snr = calc_avg_snr(&data.satellites_info);
        results.push(CriterionResult {
            name: "Average SNR".into(),
            passed: avg_snr >= self.criteria.min_avg_snr,
            expected: format!(">= {:.1} dB", self.criteria.min_avg_snr),
            actual: format!("{:.1} dB", avg_snr),
        });

        // 5. Strong satellites (SNR >= 30)
        let strong = data
            .satellites_info
            .iter()
            .filter(|s| s.snr.unwrap_or(0.0) >= 30.0)
            .count() as u32;
        results.push(CriterionResult {
            name: "Strong Sats (SNR>=30)".into(),
            passed: strong >= self.criteria.min_strong_satellites,
            expected: format!(">= {}", self.criteria.min_strong_satellites),
            actual: format!("{}", strong),
        });

        // 6. Constellation count
        let constellations: HashSet<&str> = data
            .satellites_info
            .iter()
            .map(|s| s.constellation.as_str())
            .collect();
        results.push(CriterionResult {
            name: "Constellations".into(),
            passed: constellations.len() as u32 >= self.criteria.min_constellations,
            expected: format!(">= {}", self.criteria.min_constellations),
            actual: format!("{} ({})", constellations.len(), constellations.into_iter().collect::<Vec<_>>().join(", ")),
        });

        // 7. Fix quality
        results.push(CriterionResult {
            name: "Fix Quality".into(),
            passed: has_fix,
            expected: format!(">= {}", self.criteria.min_fix_quality),
            actual: format!("{}", data.fix_quality.unwrap_or(0)),
        });

        // 8. TTFF
        let ttff = self.ttff_seconds();
        let ttff_pass = ttff.map_or(false, |t| t <= self.criteria.max_ttff_seconds as f64);
        results.push(CriterionResult {
            name: "Time to First Fix".into(),
            passed: ttff_pass || self.first_fix_time.is_some(),
            expected: format!("<= {}s", self.criteria.max_ttff_seconds),
            actual: ttff.map_or("Waiting...".into(), |t| format!("{:.1}s", t)),
        });

        // Check if all criteria pass (excluding TTFF which just needs to have happened)
        let all_pass = results.iter().all(|r| r.passed);

        if all_pass {
            // Track stability
            if self.stable_since.is_none() {
                self.stable_since = Some(Instant::now());
                log::info!("All criteria passing, stability timer started");
            }

            // Check if stable long enough
            if let Some(stable_start) = self.stable_since {
                let stable_duration = stable_start.elapsed().as_secs();
                if stable_duration >= self.criteria.stability_duration_seconds {
                    self.verdict = TestVerdict::Pass;
                    log::info!("TEST PASSED - stable for {}s", stable_duration);
                }
            }
        } else {
            // Reset stability timer if criteria fail
            if self.stable_since.is_some() {
                log::info!("Criteria no longer passing, stability timer reset");
                self.stable_since = None;
            }
        }

        // Check for overall timeout (3x TTFF limit as total test timeout)
        let total_timeout = self.criteria.max_ttff_seconds * 3 + self.criteria.stability_duration_seconds;
        if elapsed > total_timeout as f64 {
            if self.first_fix_time.is_none() {
                self.verdict = TestVerdict::TimedOut;
                log::warn!("TEST TIMED OUT - no fix acquired in {}s", elapsed);
            } else {
                self.verdict = TestVerdict::Fail;
                log::warn!("TEST FAILED - criteria not met within {}s", elapsed);
            }
        }

        self.last_criteria_results = results.clone();
        results
    }

    /// Get current test result snapshot
    pub fn get_result(&self, gps_data: Option<&GpsData>) -> TestResult {
        TestResult {
            verdict: self.verdict.clone(),
            criteria_results: self.last_criteria_results.clone(),
            ttff_seconds: self.ttff_seconds(),
            test_duration_seconds: self.elapsed_seconds(),
            device_info: self.device_info.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            best_gps_data: gps_data.cloned(),
        }
    }

    /// Abort the test
    pub fn abort(&mut self) {
        self.verdict = TestVerdict::Fail;
    }
}

/// Calculate average SNR across all satellites with signal
fn calc_avg_snr(satellites: &[crate::nmea::SatelliteInfo]) -> f32 {
    let with_snr: Vec<f32> = satellites
        .iter()
        .filter_map(|s| s.snr)
        .filter(|&snr| snr > 0.0)
        .collect();

    if with_snr.is_empty() {
        0.0
    } else {
        with_snr.iter().sum::<f32>() / with_snr.len() as f32
    }
}
