// Scout GPS Test - Factory GPS hardware verification tool

mod commands;
mod gps;
mod nmea;
mod test_criteria;
mod test_report;

use commands::AppState;
use gps::GpsManager;
use std::sync::RwLock;
use test_criteria::TestCriteria;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    // Load test criteria from config file, or use defaults
    let criteria = load_criteria();
    let results_dir = test_report::default_results_dir();

    log::info!("Scout GPS Test starting...");
    log::info!("Results directory: {}", results_dir.display());

    let app_state = AppState {
        gps_manager: GpsManager::new(),
        test_runner: RwLock::new(None),
        test_criteria: RwLock::new(criteria),
        recent_results: RwLock::new(Vec::new()),
        results_dir,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // GPS detection and connection
            commands::list_serial_ports,
            commands::auto_detect_gps,
            commands::test_gps_port,
            commands::connect_gps,
            commands::disconnect_gps,
            commands::get_gps_data,
            commands::get_gps_status,
            commands::get_nmea_buffer,
            commands::clear_nmea_buffer,
            // Test criteria
            commands::get_test_criteria,
            commands::set_test_criteria,
            commands::reset_test_criteria,
            // Test execution
            commands::start_test,
            commands::get_test_status,
            commands::abort_test,
            commands::save_test_report,
            commands::get_recent_results,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Scout GPS Test");
}

/// Load test criteria from config file, falling back to defaults
fn load_criteria() -> TestCriteria {
    let config_dir = dirs_config();
    let config_file = config_dir.join("criteria.json");

    if config_file.exists() {
        match std::fs::read_to_string(&config_file) {
            Ok(contents) => match serde_json::from_str::<TestCriteria>(&contents) {
                Ok(criteria) => {
                    log::info!("Loaded test criteria from {}", config_file.display());
                    return criteria;
                }
                Err(e) => {
                    log::warn!("Failed to parse criteria config: {}, using defaults", e);
                }
            },
            Err(e) => {
                log::warn!("Failed to read criteria config: {}, using defaults", e);
            }
        }
    }

    TestCriteria::default()
}

/// Get config directory path
fn dirs_config() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".config")
        .join("scout-gps-test")
}
