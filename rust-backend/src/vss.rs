//! Volume Shadow Copy Service (VSS) Integration
//!
//! Provides access to Windows VSS snapshots for recovering deleted files
//! from previous points in time.

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

/// Represents a Volume Shadow Copy snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VssSnapshot {
    /// Snapshot ID (GUID)
    pub id: String,
    /// Volume GUID path
    pub volume_path: String,
    /// Original volume (e.g., "C:")
    pub original_volume: String,
    /// Snapshot creation timestamp (ISO 8601)
    pub created: String,
    /// Snapshot device object path
    pub device_object: String,
    /// Whether snapshot is currently accessible
    pub available: bool,
}

/// VSS enumeration result
#[derive(Debug, Serialize, Deserialize)]
pub struct VssEnumerationResult {
    pub success: bool,
    pub snapshots: Vec<VssSnapshot>,
    pub error: Option<String>,
}

/// File recovered from VSS snapshot
#[derive(Debug, Serialize, Deserialize)]
pub struct VssFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: String,
    pub snapshot_id: String,
    pub snapshot_date: String,
}

/// Enumerates all available VSS snapshots for a given drive
pub fn enumerate_snapshots(drive_letter: &str) -> VssEnumerationResult {
    #[cfg(windows)]
    {
        match enumerate_snapshots_windows(drive_letter) {
            Ok(snapshots) => VssEnumerationResult {
                success: true,
                snapshots,
                error: None,
            },
            Err(e) => VssEnumerationResult {
                success: false,
                snapshots: vec![],
                error: Some(e),
            },
        }
    }
    
    #[cfg(not(windows))]
    {
        VssEnumerationResult {
            success: false,
            snapshots: vec![],
            error: Some("VSS is only available on Windows".to_string()),
        }
    }
}

#[cfg(windows)]
fn enumerate_snapshots_windows(drive_letter: &str) -> Result<Vec<VssSnapshot>, String> {
    use std::process::Command;
    
    // Normalize drive letter format (ensure it has colon but no backslash)
    let normalized_drive = if drive_letter.ends_with('\\') {
        drive_letter.trim_end_matches('\\')
    } else if !drive_letter.ends_with(':') {
        return Err(format!("Invalid drive letter format: {}", drive_letter));
    } else {
        drive_letter
    };
    
    // Use vssadmin to list shadow copies (requires admin privileges)
    // Correct syntax: vssadmin list shadows /for=C:\
    let for_param = format!("/for={}\\", normalized_drive);
    let output = Command::new("vssadmin")
        .args(&["list", "shadows", &for_param])
        .output()
        .map_err(|e| format!("Failed to execute vssadmin: {}. Make sure you're running as Administrator.", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if !output.status.success() {
        // Combine stdout and stderr for better error messages
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            format!("Exit code: {}", output.status.code().unwrap_or(-1))
        };
        
        // Check for common error scenarios
        let helpful_msg = if error_msg.contains("not recognized") {
            "VSS is not available on this system."
        } else if error_msg.contains("Access is denied") || error_msg.contains("administrator") {
            "Access denied. Please run RecoverPro as Administrator."
        } else if error_msg.contains("service is not started") {
            "Volume Shadow Copy service is not running. Start it from Services (services.msc)."
        } else {
            ""
        };
        
        let full_error = if helpful_msg.is_empty() {
            format!("vssadmin failed: {}", error_msg.trim())
        } else {
            format!("{} Details: {}", helpful_msg, error_msg.trim())
        };
        
        return Err(full_error);
    }
    
    // Check if no snapshots were found
    if stdout.contains("No items found") || stdout.contains("No shadow copies") {
        eprintln!("VSS: No snapshots found for {}", normalized_drive);
        return Ok(Vec::new());
    }
    
    eprintln!("VSS: Parsing vssadmin output for {}", normalized_drive);
    eprintln!("VSS: Output length: {} bytes", stdout.len());
    
    let result = parse_vssadmin_output(&stdout, normalized_drive);
    
    match &result {
        Ok(snapshots) => eprintln!("VSS: Found {} snapshots", snapshots.len()),
        Err(e) => eprintln!("VSS: Parse error: {}", e),
    }
    
    result
}

#[cfg(windows)]
fn parse_vssadmin_output(output: &str, drive_letter: &str) -> Result<Vec<VssSnapshot>, String> {
    let mut snapshots = Vec::new();
    let lines: Vec<&str> = output.lines().collect();
    
    let mut current_snapshot: Option<VssSnapshot> = None;
    let mut pending_creation_time: Option<String> = None;
    
    for line in lines {
        let line = line.trim();
        
        // Check for creation time in "Contained X shadow copies at creation time: DATE" format
        if line.contains("at creation time:") {
            if let Some(date_part) = line.split("at creation time:").nth(1) {
                pending_creation_time = Some(date_part.trim().to_string());
            }
        } else if line.starts_with("Shadow Copy ID:") {
            // Start new snapshot
            if let Some(snapshot) = current_snapshot.take() {
                snapshots.push(snapshot);
            }
            
            let id = line.replace("Shadow Copy ID:", "").trim().to_string();
            let created = pending_creation_time.take().unwrap_or_default();
            
            current_snapshot = Some(VssSnapshot {
                id,
                volume_path: String::new(),
                original_volume: drive_letter.to_string(),
                created: parse_windows_date(&created),
                device_object: String::new(),
                available: true,
            });
        } else if line.starts_with("Original Volume:") {
            if let Some(ref mut snapshot) = current_snapshot {
                snapshot.volume_path = line.replace("Original Volume:", "").trim().to_string();
            }
        } else if line.starts_with("Creation Time:") {
            // Also support standalone "Creation Time:" format for backward compatibility
            if let Some(ref mut snapshot) = current_snapshot {
                let date_str = line.replace("Creation Time:", "").trim().to_string();
                snapshot.created = parse_windows_date(&date_str);
            }
        } else if line.starts_with("Shadow Copy Volume:") {
            if let Some(ref mut snapshot) = current_snapshot {
                snapshot.device_object = line.replace("Shadow Copy Volume:", "").trim().to_string();
            }
        }
    }
    
    // Push last snapshot
    if let Some(snapshot) = current_snapshot {
        snapshots.push(snapshot);
    }
    
    Ok(snapshots)
}

#[cfg(windows)]
fn parse_windows_date(date_str: &str) -> String {
    // Parse Windows date format and convert to ISO 8601
    // Common formats:
    // - "04-02-2026 12:14:02" (DD-MM-YYYY HH:MM:SS from vssadmin)
    // - "5/15/2025 2:30:45 PM" (M/D/YYYY h:mm:ss AM/PM)
    // - "2025-05-15 14:30:45" (ISO-like)
    
    use chrono::{NaiveDateTime, DateTime, Utc};
    
    if date_str.is_empty() {
        return String::new();
    }
    
    // Try multiple date formats
    let formats = vec![
        "%d-%m-%Y %H:%M:%S",      // DD-MM-YYYY HH:MM:SS (vssadmin format)
        "%m-%d-%Y %H:%M:%S",      // MM-DD-YYYY HH:MM:SS
        "%Y-%m-%d %H:%M:%S",      // YYYY-MM-DD HH:MM:SS
        "%m/%d/%Y %I:%M:%S %p",   // M/D/YYYY h:mm:ss AM/PM
        "%d/%m/%Y %I:%M:%S %p",   // D/M/YYYY h:mm:ss AM/PM
        "%m/%d/%Y %H:%M:%S",      // M/D/YYYY HH:MM:SS
        "%d/%m/%Y %H:%M:%S",      // D/M/YYYY HH:MM:SS
    ];
    
    for format in formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, format) {
            let utc_dt: DateTime<Utc> = DateTime::from_naive_utc_and_offset(dt, Utc);
            return utc_dt.to_rfc3339();
        }
    }
    
    // Fallback: return original string
    eprintln!("Warning: Could not parse date '{}', using as-is", date_str);
    date_str.to_string()
}

/// Lists files in a VSS snapshot
pub fn list_files_in_snapshot(snapshot: &VssSnapshot, path: Option<&str>) -> Result<Vec<VssFile>, String> {
    #[cfg(windows)]
    {
        list_files_in_snapshot_windows(snapshot, path)
    }
    
    #[cfg(not(windows))]
    {
        Err("VSS is only available on Windows".to_string())
    }
}

#[cfg(windows)]
fn list_files_in_snapshot_windows(snapshot: &VssSnapshot, path: Option<&str>) -> Result<Vec<VssFile>, String> {
    let mut files = Vec::new();
    
    // VSS snapshots are accessed via \\?\GLOBALROOT\Device\HarddiskVolumeShadowCopy{N}\
    let snapshot_path = &snapshot.device_object;
    
    if snapshot_path.is_empty() {
        return Err("Invalid snapshot device object".to_string());
    }
    
    // Add trailing backslash if needed
    let base_path = if snapshot_path.ends_with('\\') {
        snapshot_path.clone()
    } else {
        format!("{}\\", snapshot_path)
    };
    
    // Append custom path if provided
    let search_path = if let Some(p) = path {
        format!("{}{}", base_path, p.trim_start_matches('\\'))
    } else {
        base_path
    };
    
    // Use walkdir to recursively scan
    let walker = WalkDir::new(&search_path)
        .max_depth(10)
        .follow_links(false);
    
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                let path_str = entry.path().to_string_lossy().to_string();
                let name = entry.file_name().to_string_lossy().to_string();
                
                let modified = if let Ok(modified_time) = metadata.modified() {
                    let datetime: chrono::DateTime<chrono::Utc> = modified_time.into();
                    datetime.to_rfc3339()
                } else {
                    String::new()
                };
                
                files.push(VssFile {
                    path: path_str,
                    name,
                    size: metadata.len(),
                    modified,
                    snapshot_id: snapshot.id.clone(),
                    snapshot_date: snapshot.created.clone(),
                });
            }
        }
    }
    
    Ok(files)
}

/// Recovers a file from a VSS snapshot to a destination
pub fn recover_from_snapshot(
    _snapshot: &VssSnapshot,
    source_path: &str,
    destination_path: &str,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::fs;
        
        // Copy file from snapshot to destination
        fs::copy(source_path, destination_path)
            .map_err(|e| format!("Failed to recover file: {}", e))?;
        
        Ok(())
    }
    
    #[cfg(not(windows))]
    {
        Err("VSS is only available on Windows".to_string())
    }
}

/// Checks if VSS is available on the system
pub fn is_vss_available() -> bool {
    #[cfg(windows)]
    {
        use std::process::Command;
        
        // Check if vssadmin is accessible and we have admin rights
        match Command::new("vssadmin")
            .args(&["list", "shadows"])
            .output() {
            Ok(output) => {
                // If we can run vssadmin list shadows, VSS is available
                // Even if there are no shadows, the command should succeed
                output.status.success() || 
                String::from_utf8_lossy(&output.stdout).contains("vssadmin") ||
                String::from_utf8_lossy(&output.stderr).contains("vssadmin")
            },
            Err(_) => false
        }
    }
    
    #[cfg(not(windows))]
    {
        false
    }
}

/// Gets the count of available snapshots for a drive
pub fn get_snapshot_count(drive_letter: &str) -> usize {
    let result = enumerate_snapshots(drive_letter);
    result.snapshots.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vss_available() {
        let available = is_vss_available();
        println!("VSS Available: {}", available);
    }
    
    #[test]
    #[cfg(windows)]
    fn test_enumerate_snapshots() {
        let result = enumerate_snapshots("C:");
        println!("Enumeration result: {:?}", result);
        assert!(result.success || result.error.is_some());
    }
}
