// Tauri command handlers for GPS operations and test engine

use crate::gps::{DetectedPort, GpsManager, GpsSourceStatus};
use crate::nmea::GpsData;
use crate::test_criteria::{CriterionResult, DeviceInfo, TestCriteria, TestResult, TestRunner, TestVerdict};
use crate::test_report;
use serde::Serialize;
use std::sync::RwLock;
use tauri::State;

/// Standard command response wrapper
#[derive(Debug, Serialize)]
pub struct CommandResult<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> CommandResult<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(msg.into()) }
    }
}

/// Application state
pub struct AppState {
    pub gps_manager: GpsManager,
    pub test_runner: RwLock<Option<TestRunner>>,
    pub test_criteria: RwLock<TestCriteria>,
    pub recent_results: RwLock<Vec<TestResult>>,
    pub results_dir: std::path::PathBuf,
}

// ============ GPS Commands ============

#[tauri::command]
pub fn list_serial_ports() -> CommandResult<Vec<DetectedPort>> {
    match GpsManager::list_serial_ports() {
        Ok(ports) => CommandResult::ok(ports),
        Err(e) => CommandResult::err(e.to_string()),
    }
}

#[tauri::command]
pub fn auto_detect_gps() -> CommandResult<(DetectedPort, u32)> {
    match GpsManager::auto_detect_gps() {
        Ok(result) => CommandResult::ok(result),
        Err(e) => CommandResult::err(e.to_string()),
    }
}

#[tauri::command]
pub fn test_gps_port(port_name: String, baud_rate: u32) -> CommandResult<bool> {
    match GpsManager::test_port(&port_name, baud_rate, 3000) {
        Ok(result) => CommandResult::ok(result),
        Err(e) => CommandResult::err(e.to_string()),
    }
}

#[tauri::command]
pub fn connect_gps(state: State<'_, AppState>, port_name: String, baud_rate: u32) -> CommandResult<bool> {
    match state.gps_manager.connect(&port_name, baud_rate) {
        Ok(()) => CommandResult::ok(true),
        Err(e) => CommandResult::err(e.to_string()),
    }
}

#[tauri::command]
pub fn disconnect_gps(state: State<'_, AppState>) -> CommandResult<bool> {
    state.gps_manager.disconnect();
    CommandResult::ok(true)
}

#[tauri::command]
pub fn get_gps_data(state: State<'_, AppState>) -> CommandResult<GpsData> {
    CommandResult::ok(state.gps_manager.get_data())
}

#[tauri::command]
pub fn get_gps_status(state: State<'_, AppState>) -> CommandResult<GpsSourceStatus> {
    CommandResult::ok(state.gps_manager.get_status())
}

#[tauri::command]
pub fn get_nmea_buffer(state: State<'_, AppState>) -> CommandResult<Vec<String>> {
    CommandResult::ok(state.gps_manager.get_nmea_buffer())
}

#[tauri::command]
pub fn clear_nmea_buffer(state: State<'_, AppState>) -> CommandResult<bool> {
    state.gps_manager.clear_nmea_buffer();
    CommandResult::ok(true)
}

// ============ Test Criteria Commands ============

#[tauri::command]
pub fn get_test_criteria(state: State<'_, AppState>) -> CommandResult<TestCriteria> {
    let criteria = state.test_criteria.read().unwrap().clone();
    CommandResult::ok(criteria)
}

#[tauri::command]
pub fn set_test_criteria(state: State<'_, AppState>, criteria: TestCriteria) -> CommandResult<bool> {
    *state.test_criteria.write().unwrap() = criteria;
    CommandResult::ok(true)
}

#[tauri::command]
pub fn reset_test_criteria(state: State<'_, AppState>) -> CommandResult<TestCriteria> {
    let defaults = TestCriteria::default();
    *state.test_criteria.write().unwrap() = defaults.clone();
    CommandResult::ok(defaults)
}

// ============ Test Execution Commands ============

#[tauri::command]
pub fn start_test(state: State<'_, AppState>) -> CommandResult<bool> {
    let status = state.gps_manager.get_status();

    // Get device info from current GPS connection
    let port_name = match status.port_name {
        Some(ref name) => name.clone(),
        None => return CommandResult::err("No GPS connected. Connect a GPS device first."),
    };

    // Try to get device details from port list
    let device_info = match GpsManager::list_serial_ports() {
        Ok(ports) => {
            if let Some(port) = ports.iter().find(|p| p.port_name == port_name) {
                DeviceInfo {
                    port_name: port.port_name.clone(),
                    port_type: port.port_type.clone(),
                    manufacturer: port.manufacturer.clone(),
                    product: port.product.clone(),
                    serial_number: port.serial_number.clone(),
                }
            } else {
                DeviceInfo {
                    port_name,
                    port_type: "Unknown".into(),
                    manufacturer: None,
                    product: None,
                    serial_number: None,
                }
            }
        }
        Err(_) => DeviceInfo {
            port_name,
            port_type: "Unknown".into(),
            manufacturer: None,
            product: None,
            serial_number: None,
        },
    };

    let criteria = state.test_criteria.read().unwrap().clone();
    let mut runner = TestRunner::new(criteria, device_info);
    runner.start();

    *state.test_runner.write().unwrap() = Some(runner);
    CommandResult::ok(true)
}

#[tauri::command]
pub fn get_test_status(state: State<'_, AppState>) -> CommandResult<TestResult> {
    let mut runner_lock = state.test_runner.write().unwrap();

    match runner_lock.as_mut() {
        Some(runner) => {
            // If test is running, evaluate current GPS data
            if runner.verdict == TestVerdict::Running {
                let gps_data = state.gps_manager.get_data();
                runner.evaluate(&gps_data);
            }

            let gps_data = state.gps_manager.get_data();
            let result = runner.get_result(Some(&gps_data));
            CommandResult::ok(result)
        }
        None => {
            // No test running
            CommandResult::ok(TestResult {
                verdict: TestVerdict::NotStarted,
                criteria_results: Vec::new(),
                ttff_seconds: None,
                test_duration_seconds: 0.0,
                device_info: DeviceInfo {
                    port_name: "None".into(),
                    port_type: "None".into(),
                    manufacturer: None,
                    product: None,
                    serial_number: None,
                },
                timestamp: chrono::Utc::now().to_rfc3339(),
                best_gps_data: None,
            })
        }
    }
}

#[tauri::command]
pub fn abort_test(state: State<'_, AppState>) -> CommandResult<bool> {
    let mut runner_lock = state.test_runner.write().unwrap();
    if let Some(runner) = runner_lock.as_mut() {
        runner.abort();
    }
    CommandResult::ok(true)
}

#[tauri::command]
pub fn save_test_report(state: State<'_, AppState>) -> CommandResult<String> {
    let runner_lock = state.test_runner.read().unwrap();

    match runner_lock.as_ref() {
        Some(runner) => {
            let gps_data = state.gps_manager.get_data();
            let result = runner.get_result(Some(&gps_data));

            // Save to recent results
            {
                let mut recent = state.recent_results.write().unwrap();
                recent.push(result.clone());
                // Keep last 50
                if recent.len() > 50 {
                    recent.remove(0);
                }
            }

            // Save to file
            match test_report::save_report(&result, &state.results_dir) {
                Ok(path) => CommandResult::ok(path.display().to_string()),
                Err(e) => CommandResult::err(format!("Failed to save report: {}", e)),
            }
        }
        None => CommandResult::err("No test results to save"),
    }
}

#[tauri::command]
pub fn get_recent_results(state: State<'_, AppState>) -> CommandResult<Vec<TestResult>> {
    let recent = state.recent_results.read().unwrap().clone();
    CommandResult::ok(recent)
}
