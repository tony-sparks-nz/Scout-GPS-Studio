// Optimization engine: before/after GPS performance comparison
//
// State machine: Idle -> IdentifyingChip -> CollectingBaseline -> ApplyingProfile
//                -> Stabilizing -> CollectingResult -> Complete | Error

use crate::nmea::GpsData;
use crate::ubx_config::{self, UbloxChipInfo, UbloxSeries};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;

// Phase durations in seconds
const BASELINE_DURATION: u64 = 30;
const STABILIZATION_DURATION: u64 = 30;
const RESULT_DURATION: u64 = 30;
const MON_VER_TIMEOUT: u64 = 5;

// ============ Types ============

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OptimizePhase {
    Idle,
    IdentifyingChip,
    CollectingBaseline,
    ApplyingProfile,
    Stabilizing,
    CollectingResult,
    Complete,
    Error,
}

/// Averaged GPS performance metrics over a sample window
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceSnapshot {
    pub avg_hdop: f32,
    pub avg_satellites: f32,
    pub avg_snr: f32,
    pub constellation_count: u32,
    pub constellations: Vec<String>,
    pub avg_fix_quality: f32,
    pub sample_count: u32,
}

/// Before/after comparison report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub chip_info: UbloxChipInfo,
    pub profile_applied: String,
    pub before: PerformanceSnapshot,
    pub after: PerformanceSnapshot,
    pub hdop_improvement_pct: f32,
    pub satellite_improvement_pct: f32,
    pub snr_improvement_pct: f32,
    pub constellation_improvement: i32,
    pub timestamp: String,
}

/// Status sent to the frontend each poll cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeStatus {
    pub phase: OptimizePhase,
    pub chip_info: Option<UbloxChipInfo>,
    pub progress_seconds: f32,
    pub phase_duration_seconds: f32,
    pub error: Option<String>,
    pub report: Option<OptimizationReport>,
    pub baseline_snapshot: Option<PerformanceSnapshot>,
}

// ============ Metrics Collector ============

struct MetricsCollector {
    hdop_samples: Vec<f32>,
    satellite_samples: Vec<u32>,
    snr_samples: Vec<f32>,
    fix_quality_samples: Vec<u8>,
    constellation_sets: Vec<HashSet<String>>,
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            hdop_samples: Vec::new(),
            satellite_samples: Vec::new(),
            snr_samples: Vec::new(),
            fix_quality_samples: Vec::new(),
            constellation_sets: Vec::new(),
        }
    }

    fn add_sample(&mut self, data: &GpsData) {
        if let Some(hdop) = data.hdop {
            self.hdop_samples.push(hdop);
        }
        if let Some(sats) = data.satellites {
            self.satellite_samples.push(sats);
        }
        if let Some(fq) = data.fix_quality {
            self.fix_quality_samples.push(fq);
        }

        // Average SNR across satellites with signal
        let snrs: Vec<f32> = data
            .satellites_info
            .iter()
            .filter_map(|s| s.snr)
            .filter(|&s| s > 0.0)
            .collect();
        if !snrs.is_empty() {
            let avg = snrs.iter().sum::<f32>() / snrs.len() as f32;
            self.snr_samples.push(avg);
        }

        let consts: HashSet<String> = data
            .satellites_info
            .iter()
            .map(|s| s.constellation.clone())
            .collect();
        if !consts.is_empty() {
            self.constellation_sets.push(consts);
        }
    }

    fn snapshot(&self) -> PerformanceSnapshot {
        let avg_hdop = if self.hdop_samples.is_empty() {
            0.0
        } else {
            self.hdop_samples.iter().sum::<f32>() / self.hdop_samples.len() as f32
        };

        let avg_sats = if self.satellite_samples.is_empty() {
            0.0
        } else {
            self.satellite_samples.iter().sum::<u32>() as f32
                / self.satellite_samples.len() as f32
        };

        let avg_snr = if self.snr_samples.is_empty() {
            0.0
        } else {
            self.snr_samples.iter().sum::<f32>() / self.snr_samples.len() as f32
        };

        let avg_fq = if self.fix_quality_samples.is_empty() {
            0.0
        } else {
            self.fix_quality_samples
                .iter()
                .map(|&x| x as f32)
                .sum::<f32>()
                / self.fix_quality_samples.len() as f32
        };

        let all_consts: HashSet<String> = self
            .constellation_sets
            .iter()
            .flat_map(|s| s.iter().cloned())
            .collect();

        let mut sorted_consts: Vec<String> = all_consts.into_iter().collect();
        sorted_consts.sort();

        PerformanceSnapshot {
            avg_hdop,
            avg_satellites: avg_sats,
            avg_snr,
            constellation_count: sorted_consts.len() as u32,
            constellations: sorted_consts,
            avg_fix_quality: avg_fq,
            sample_count: self
                .hdop_samples
                .len()
                .max(self.satellite_samples.len()) as u32,
        }
    }
}

// ============ Optimizer State Machine ============

pub struct UbxOptimizer {
    pub phase: OptimizePhase,
    pub chip_info: Option<UbloxChipInfo>,
    phase_start: Option<Instant>,
    baseline_collector: MetricsCollector,
    result_collector: MetricsCollector,
    baseline_snapshot: Option<PerformanceSnapshot>,
    report: Option<OptimizationReport>,
    error: Option<String>,
    /// Commands queued for GpsManager to send
    pub pending_commands: Vec<Vec<u8>>,
    /// True when waiting for MON-VER binary response
    pub awaiting_mon_ver: bool,
}

impl UbxOptimizer {
    pub fn new() -> Self {
        Self {
            phase: OptimizePhase::Idle,
            chip_info: None,
            phase_start: None,
            baseline_collector: MetricsCollector::new(),
            result_collector: MetricsCollector::new(),
            baseline_snapshot: None,
            report: None,
            error: None,
            pending_commands: Vec::new(),
            awaiting_mon_ver: false,
        }
    }

    /// Begin the optimization process
    pub fn start(&mut self) {
        *self = Self::new();
        self.phase = OptimizePhase::IdentifyingChip;
        self.phase_start = Some(Instant::now());
        self.pending_commands.push(ubx_config::build_mon_ver_poll());
        self.awaiting_mon_ver = true;
    }

    /// Called when a UBX-MON-VER response is received from the reader thread
    pub fn on_mon_ver_response(&mut self, payload: &[u8]) {
        self.awaiting_mon_ver = false;

        match ubx_config::parse_mon_ver(payload) {
            Some(info) => {
                log::info!(
                    "Chip identified: {} (HW: {}, series: {})",
                    info.chip_name,
                    info.hw_version,
                    info.series
                );
                self.chip_info = Some(info);
                self.phase = OptimizePhase::CollectingBaseline;
                self.phase_start = Some(Instant::now());
            }
            None => {
                self.error = Some("Failed to parse MON-VER response".to_string());
                self.phase = OptimizePhase::Error;
            }
        }
    }

    /// Called when MON-VER poll times out
    fn on_mon_ver_timeout(&mut self) {
        self.awaiting_mon_ver = false;
        self.error = Some(
            "Could not identify chip — device may not be u-blox or UBX protocol is disabled"
                .to_string(),
        );
        self.phase = OptimizePhase::Error;
    }

    /// Feed a GPS data sample; called each poll cycle (~500ms).
    /// Returns true if there are pending commands to send.
    pub fn tick(&mut self, data: &GpsData) -> bool {
        let elapsed = self
            .phase_start
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);

        match self.phase {
            OptimizePhase::IdentifyingChip => {
                if elapsed >= MON_VER_TIMEOUT && self.awaiting_mon_ver {
                    self.on_mon_ver_timeout();
                }
            }
            OptimizePhase::CollectingBaseline => {
                self.baseline_collector.add_sample(data);
                if elapsed >= BASELINE_DURATION {
                    self.baseline_snapshot = Some(self.baseline_collector.snapshot());
                    log::info!(
                        "Baseline collected ({} samples): HDOP={:.2}, Sats={:.1}, SNR={:.1}",
                        self.baseline_snapshot.as_ref().unwrap().sample_count,
                        self.baseline_snapshot.as_ref().unwrap().avg_hdop,
                        self.baseline_snapshot.as_ref().unwrap().avg_satellites,
                        self.baseline_snapshot.as_ref().unwrap().avg_snr,
                    );

                    let series = self
                        .chip_info
                        .as_ref()
                        .map(|c| c.series.clone())
                        .unwrap_or(UbloxSeries::Unknown);
                    self.pending_commands = ubx_config::get_optimization_commands(&series);
                    self.phase = OptimizePhase::ApplyingProfile;
                    self.phase_start = Some(Instant::now());
                    return true;
                }
            }
            OptimizePhase::ApplyingProfile => {
                if self.pending_commands.is_empty() {
                    log::info!("Optimization profile applied, stabilizing...");
                    self.phase = OptimizePhase::Stabilizing;
                    self.phase_start = Some(Instant::now());
                }
            }
            OptimizePhase::Stabilizing => {
                if elapsed >= STABILIZATION_DURATION {
                    self.phase = OptimizePhase::CollectingResult;
                    self.phase_start = Some(Instant::now());
                }
            }
            OptimizePhase::CollectingResult => {
                self.result_collector.add_sample(data);
                if elapsed >= RESULT_DURATION {
                    let after = self.result_collector.snapshot();
                    let before = self.baseline_snapshot.clone().unwrap_or_default();
                    self.report = Some(self.build_report(&before, &after));
                    self.phase = OptimizePhase::Complete;
                    log::info!("Optimization complete — report generated");
                }
            }
            _ => {}
        }

        !self.pending_commands.is_empty()
    }

    fn build_report(
        &self,
        before: &PerformanceSnapshot,
        after: &PerformanceSnapshot,
    ) -> OptimizationReport {
        // HDOP: lower is better, so positive improvement = HDOP decreased
        let hdop_imp = if before.avg_hdop > 0.0 {
            ((before.avg_hdop - after.avg_hdop) / before.avg_hdop) * 100.0
        } else {
            0.0
        };

        let sat_imp = if before.avg_satellites > 0.0 {
            ((after.avg_satellites - before.avg_satellites) / before.avg_satellites) * 100.0
        } else {
            0.0
        };

        let snr_imp = if before.avg_snr > 0.0 {
            ((after.avg_snr - before.avg_snr) / before.avg_snr) * 100.0
        } else {
            0.0
        };

        let const_delta = after.constellation_count as i32 - before.constellation_count as i32;

        let series = self
            .chip_info
            .as_ref()
            .map(|c| &c.series)
            .unwrap_or(&UbloxSeries::Unknown);

        OptimizationReport {
            chip_info: self.chip_info.clone().unwrap_or(UbloxChipInfo {
                sw_version: "Unknown".into(),
                hw_version: "Unknown".into(),
                extensions: vec![],
                series: UbloxSeries::Unknown,
                chip_name: "Unknown".into(),
            }),
            profile_applied: ubx_config::profile_name(series).to_string(),
            before: before.clone(),
            after: after.clone(),
            hdop_improvement_pct: hdop_imp,
            satellite_improvement_pct: sat_imp,
            snr_improvement_pct: snr_imp,
            constellation_improvement: const_delta,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Get current status for the frontend
    pub fn get_status(&self) -> OptimizeStatus {
        let (progress, duration) = match self.phase {
            OptimizePhase::IdentifyingChip => (self.phase_elapsed_secs(), MON_VER_TIMEOUT as f32),
            OptimizePhase::CollectingBaseline => {
                (self.phase_elapsed_secs(), BASELINE_DURATION as f32)
            }
            OptimizePhase::ApplyingProfile => (self.phase_elapsed_secs(), 3.0),
            OptimizePhase::Stabilizing => {
                (self.phase_elapsed_secs(), STABILIZATION_DURATION as f32)
            }
            OptimizePhase::CollectingResult => (self.phase_elapsed_secs(), RESULT_DURATION as f32),
            _ => (0.0, 0.0),
        };

        OptimizeStatus {
            phase: self.phase.clone(),
            chip_info: self.chip_info.clone(),
            progress_seconds: progress,
            phase_duration_seconds: duration,
            error: self.error.clone(),
            report: self.report.clone(),
            baseline_snapshot: self.baseline_snapshot.clone(),
        }
    }

    fn phase_elapsed_secs(&self) -> f32 {
        self.phase_start
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0)
    }

    /// Reset to idle
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nmea::SatelliteInfo;

    fn make_gps_data(
        hdop: f32,
        satellites: u32,
        fix_quality: u8,
        sat_infos: Vec<SatelliteInfo>,
    ) -> GpsData {
        GpsData {
            hdop: Some(hdop),
            satellites: Some(satellites),
            fix_quality: Some(fix_quality),
            satellites_info: sat_infos,
            ..GpsData::default()
        }
    }

    fn make_sat(constellation: &str, snr: f32) -> SatelliteInfo {
        SatelliteInfo {
            prn: 1,
            elevation: Some(45.0),
            azimuth: Some(180.0),
            snr: Some(snr),
            constellation: constellation.to_string(),
        }
    }

    #[test]
    fn test_metrics_collector_averaging() {
        let mut collector = MetricsCollector::new();

        let data1 = make_gps_data(
            1.5,
            8,
            1,
            vec![make_sat("GPS", 35.0), make_sat("GLONASS", 25.0)],
        );
        let data2 = make_gps_data(
            2.5,
            10,
            1,
            vec![
                make_sat("GPS", 40.0),
                make_sat("GLONASS", 30.0),
                make_sat("Galileo", 20.0),
            ],
        );

        collector.add_sample(&data1);
        collector.add_sample(&data2);

        let snap = collector.snapshot();
        assert!((snap.avg_hdop - 2.0).abs() < 0.01);
        assert!((snap.avg_satellites - 9.0).abs() < 0.01);
        assert_eq!(snap.sample_count, 2);
        assert_eq!(snap.constellation_count, 3); // GPS, GLONASS, Galileo
    }

    #[test]
    fn test_metrics_collector_empty() {
        let collector = MetricsCollector::new();
        let snap = collector.snapshot();
        assert_eq!(snap.avg_hdop, 0.0);
        assert_eq!(snap.avg_satellites, 0.0);
        assert_eq!(snap.sample_count, 0);
    }

    #[test]
    fn test_optimizer_starts_in_idle() {
        let opt = UbxOptimizer::new();
        assert_eq!(opt.phase, OptimizePhase::Idle);
        assert!(opt.pending_commands.is_empty());
    }

    #[test]
    fn test_optimizer_start_queues_mon_ver() {
        let mut opt = UbxOptimizer::new();
        opt.start();
        assert_eq!(opt.phase, OptimizePhase::IdentifyingChip);
        assert!(opt.awaiting_mon_ver);
        assert_eq!(opt.pending_commands.len(), 1);
        // Should be a MON-VER poll
        let cmd = &opt.pending_commands[0];
        assert_eq!(cmd[2], 0x0A); // MON class
        assert_eq!(cmd[3], 0x04); // VER id
    }

    #[test]
    fn test_optimizer_mon_ver_response_transitions_to_baseline() {
        let mut opt = UbxOptimizer::new();
        opt.start();

        // Simulate MON-VER response for M8
        let mut payload = vec![0u8; 40];
        // HW version at bytes 30-39
        payload[30..38].copy_from_slice(b"00080000");

        opt.on_mon_ver_response(&payload);
        assert_eq!(opt.phase, OptimizePhase::CollectingBaseline);
        assert!(!opt.awaiting_mon_ver);
        assert!(opt.chip_info.is_some());
        assert_eq!(opt.chip_info.as_ref().unwrap().series, UbloxSeries::Series8);
    }

    #[test]
    fn test_improvement_calculation_hdop_decrease_is_positive() {
        // HDOP going from 3.0 to 1.5 = 50% improvement
        let before = PerformanceSnapshot {
            avg_hdop: 3.0,
            avg_satellites: 6.0,
            avg_snr: 25.0,
            constellation_count: 1,
            constellations: vec!["GPS".into()],
            avg_fix_quality: 1.0,
            sample_count: 10,
        };
        let after = PerformanceSnapshot {
            avg_hdop: 1.5,
            avg_satellites: 10.0,
            avg_snr: 30.0,
            constellation_count: 3,
            constellations: vec!["GPS".into(), "GLONASS".into(), "Galileo".into()],
            avg_fix_quality: 1.0,
            sample_count: 10,
        };

        let mut opt = UbxOptimizer::new();
        opt.chip_info = Some(UbloxChipInfo {
            sw_version: "test".into(),
            hw_version: "00080000".into(),
            extensions: vec![],
            series: UbloxSeries::Series8,
            chip_name: "test".into(),
        });

        let report = opt.build_report(&before, &after);
        assert!((report.hdop_improvement_pct - 50.0).abs() < 0.1);
        assert!((report.satellite_improvement_pct - 66.7).abs() < 0.1);
        assert!((report.snr_improvement_pct - 20.0).abs() < 0.1);
        assert_eq!(report.constellation_improvement, 2);
    }

    #[test]
    fn test_optimizer_reset() {
        let mut opt = UbxOptimizer::new();
        opt.start();
        assert_eq!(opt.phase, OptimizePhase::IdentifyingChip);
        opt.reset();
        assert_eq!(opt.phase, OptimizePhase::Idle);
        assert!(opt.pending_commands.is_empty());
    }
}
