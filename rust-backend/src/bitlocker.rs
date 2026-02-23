//! BitLocker Detection and Management Module
//! Handles detection of BitLocker-encrypted drives and provides unlock functionality

use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BitLockerStatus {
    pub drive: String,
    pub is_encrypted: bool,
    pub is_locked: bool,
    pub protection_status: String,
    pub encryption_percentage: u8,
    pub encryption_method: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BitLockerUnlockResult {
    pub success: bool,
    pub message: String,
}

/// Check if a drive is BitLocker encrypted and its lock status
pub fn get_bitlocker_status(drive_letter: &str) -> BitLockerStatus {
    let drive = drive_letter.trim_end_matches('\\').trim_end_matches(':');
    let drive_with_colon = format!("{}:", drive);
    
    // Use manage-bde to check BitLocker status
    let output = Command::new("manage-bde")
        .args(["-status", &drive_with_colon])
        .output();
    
    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);

            // If manage-bde returned no output at all (e.g. ran without admin on some systems)
            // fall back to "not encrypted / unknown" rather than misreporting.
            if stdout.trim().is_empty() {
                return BitLockerStatus {
                    drive: drive_with_colon,
                    is_encrypted: false,
                    is_locked: false,
                    protection_status: "Unable to determine".to_string(),
                    encryption_percentage: 0,
                    encryption_method: "Unknown".to_string(),
                };
            }

            // Use the dedicated percentage parser — it correctly handles "0.0%", "100.0%", etc.
            let encryption_percentage = extract_percentage(&stdout);

            // A drive is NOT BitLocker-encrypted when any of these are true:
            //   1. "BitLocker Version: … None"  — volume was never encrypted
            //   2. "Conversion Status: … Fully Decrypted" — decryption completed
            //   3. Parsed encryption percentage is 0
            let bitlocker_version_none = stdout
                .lines()
                .any(|l| l.contains("BitLocker Version:") && l.trim_end().ends_with("None"));
            let fully_decrypted = stdout
                .lines()
                .any(|l| l.contains("Conversion Status:") && l.contains("Fully Decrypted"));

            let is_encrypted = !bitlocker_version_none && !fully_decrypted && encryption_percentage > 0;

            // "Lock Status:" line contains "Locked" only on actually-locked volumes.
            let is_locked = stdout
                .lines()
                .any(|l| l.contains("Lock Status:") && l.contains("Locked"))
                || stderr.to_lowercase().contains("locked");

            let protection_status = if stdout.contains("Protection On") {
                "Protection On"
            } else if stdout.contains("Protection Off") {
                "Protection Off"
            } else {
                "Unknown"
            }
            .to_string();

            let encryption_method = extract_encryption_method(&stdout);

            BitLockerStatus {
                drive: drive_with_colon,
                is_encrypted,
                is_locked,
                protection_status,
                encryption_percentage,
                encryption_method,
            }
        }
        Err(_) => {
            // manage-bde not available or failed — assume not encrypted
            BitLockerStatus {
                drive: drive_with_colon,
                is_encrypted: false,
                is_locked: false,
                protection_status: "Unable to determine".to_string(),
                encryption_percentage: 0,
                encryption_method: "Unknown".to_string(),
            }
        }
    }
}

/// Unlock a BitLocker-encrypted drive using a password
pub fn unlock_with_password(drive_letter: &str, password: &str) -> BitLockerUnlockResult {
    let drive = drive_letter.trim_end_matches('\\').trim_end_matches(':');
    let drive_with_colon = format!("{}:", drive);
    
    let output = Command::new("manage-bde")
        .args(["-unlock", &drive_with_colon, "-password", password])
        .output();
    
    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            
            if result.status.success() || stdout.contains("successfully unlocked") {
                BitLockerUnlockResult {
                    success: true,
                    message: format!("Drive {} unlocked successfully", drive_with_colon),
                }
            } else {
                BitLockerUnlockResult {
                    success: false,
                    message: format!("Failed to unlock: {}", stderr.trim()),
                }
            }
        }
        Err(e) => BitLockerUnlockResult {
            success: false,
            message: format!("Failed to execute unlock command: {}", e),
        },
    }
}

/// Unlock a BitLocker-encrypted drive using a recovery key
pub fn unlock_with_recovery_key(drive_letter: &str, recovery_key: &str) -> BitLockerUnlockResult {
    let drive = drive_letter.trim_end_matches('\\').trim_end_matches(':');
    let drive_with_colon = format!("{}:", drive);
    
    let output = Command::new("manage-bde")
        .args(["-unlock", &drive_with_colon, "-recoverypassword", recovery_key])
        .output();
    
    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            
            if result.status.success() || stdout.contains("successfully unlocked") {
                BitLockerUnlockResult {
                    success: true,
                    message: format!("Drive {} unlocked with recovery key", drive_with_colon),
                }
            } else {
                BitLockerUnlockResult {
                    success: false,
                    message: format!("Failed to unlock: {}", stderr.trim()),
                }
            }
        }
        Err(e) => BitLockerUnlockResult {
            success: false,
            message: format!("Failed to execute unlock command: {}", e),
        },
    }
}

/// Lock a BitLocker-encrypted drive
pub fn lock_drive(drive_letter: &str) -> BitLockerUnlockResult {
    let drive = drive_letter.trim_end_matches('\\').trim_end_matches(':');
    let drive_with_colon = format!("{}:", drive);
    
    let output = Command::new("manage-bde")
        .args(["-lock", &drive_with_colon, "-forcedismount"])
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                BitLockerUnlockResult {
                    success: true,
                    message: format!("Drive {} locked successfully", drive_with_colon),
                }
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                BitLockerUnlockResult {
                    success: false,
                    message: format!("Failed to lock: {}", stderr.trim()),
                }
            }
        }
        Err(e) => BitLockerUnlockResult {
            success: false,
            message: format!("Failed to execute lock command: {}", e),
        },
    }
}

/// Check if the program is running with administrator privileges
pub fn is_admin() -> bool {
    #[cfg(windows)]
    {
        use std::ptr;
        use winapi::um::processthreadsapi::GetCurrentProcess;
        use winapi::um::processthreadsapi::OpenProcessToken;
        use winapi::um::securitybaseapi::GetTokenInformation;
        use winapi::um::winnt::{TokenElevation, HANDLE, TOKEN_ELEVATION, TOKEN_QUERY};
        
        unsafe {
            let mut token_handle: HANDLE = ptr::null_mut();
            let process_handle = GetCurrentProcess();
            
            if OpenProcessToken(process_handle, TOKEN_QUERY, &mut token_handle) == 0 {
                return false;
            }
            
            let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
            let mut size: u32 = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
            
            let result = GetTokenInformation(
                token_handle,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                size,
                &mut size,
            );
            
            winapi::um::handleapi::CloseHandle(token_handle);
            
            result != 0 && elevation.TokenIsElevated != 0
        }
    }
    
    #[cfg(not(windows))]
    {
        false
    }
}

fn extract_percentage(output: &str) -> u8 {
    for line in output.lines() {
        if line.contains("Percentage Encrypted:") {
            if let Some(pct) = line.split(':').nth(1) {
                let pct = pct.trim().trim_end_matches('%').trim();
                if let Ok(val) = pct.parse::<f32>() {
                    return val as u8;
                }
            }
        }
    }
    0
}

fn extract_encryption_method(output: &str) -> String {
    for line in output.lines() {
        if line.contains("Encryption Method:") {
            if let Some(method) = line.split(':').nth(1) {
                return method.trim().to_string();
            }
        }
    }
    "Unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_admin() {
        // Just ensure it runs without crashing
        let _ = is_admin();
    }
}
