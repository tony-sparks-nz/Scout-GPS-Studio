// Test report generation - saves JSON per test for factory traceability

use crate::test_criteria::TestResult;
use std::path::{Path, PathBuf};

/// Save a test result as a JSON file
pub fn save_report(result: &TestResult, output_dir: &Path) -> Result<PathBuf, std::io::Error> {
    // Ensure output directory exists
    std::fs::create_dir_all(output_dir)?;

    let serial = result
        .device_info
        .serial_number
        .as_deref()
        .unwrap_or("unknown");

    // Sanitize timestamp for filename
    let ts = result.timestamp.replace(':', "-").replace('.', "-");
    let filename = format!("gps-test_{}_{}.json", serial, ts);
    let path = output_dir.join(filename);

    let json = serde_json::to_string_pretty(result)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    std::fs::write(&path, json)?;
    log::info!("Test report saved to: {}", path.display());

    Ok(path)
}

/// Get the default results directory
pub fn default_results_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("scout-gps-results")
}
