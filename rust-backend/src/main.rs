//! RecoverPro - Professional Data Recovery Backend
//! 
//! A professional-grade data recovery engine featuring:
//! - NTFS MFT parsing for deleted file detection
//! - File signature carving for deep recovery
//! - Raw disk sector access
//! - BitLocker encrypted drive support
//! - Volume Shadow Copy (VSS) snapshot recovery
//! 
//! Requires Administrator privileges for raw disk access.

mod bitlocker;
mod disk_reader;
mod file_carver;
mod filesystem_disk_reader;
mod filesystem_recovery_engine;
mod ntfs_parser;
mod recovery_engine;
mod vss;

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use crate::bitlocker::{
    get_bitlocker_status, is_admin, lock_drive, unlock_with_password, unlock_with_recovery_key,
};
use crate::recovery_engine::{perform_scan, recover_file as recover_deleted_file};
use crate::filesystem_recovery_engine::FileSystemRecoveryEngine;

#[derive(Serialize, Deserialize, Debug)]
struct FileInfo {
    path: String,
    name: String,
    size: u64,
    extension: String,
    modified: String,
    is_deleted: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScanResult {
    files: Vec<FileInfo>,
    total_size: u64,
    total_files: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct DriveInfo {
    letter: String,
    label: String,
    total_space: u64,
    free_space: u64,
    is_bitlocker: bool,
    is_locked: bool,
    filesystem: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AdminStatus {
    is_admin: bool,
    message: String,
}

fn get_drives() -> Vec<DriveInfo> {
    let mut drives = Vec::new();
    
    // Scan for Windows drive letters A-Z
    for letter in b'A'..=b'Z' {
        let drive_letter = format!("{}:", letter as char);
        let drive_path = format!("{}\\", drive_letter);
        
        if Path::new(&drive_path).exists() {
            // Try to get disk info
            let label = get_drive_label(&drive_letter);
            let (total, free) = get_drive_space(&drive_path);
            let filesystem = get_filesystem(&drive_letter);
            
            // Check BitLocker status
            let bl_status = get_bitlocker_status(&drive_letter);
            
            drives.push(DriveInfo {
                letter: drive_letter,
                label,
                total_space: total,
                free_space: free,
                is_bitlocker: bl_status.is_encrypted,
                is_locked: bl_status.is_locked,
                filesystem,
            });
        }
    }
    
    drives
}

/// Perform scan using FileSystem backend (for encrypted drives)
/// Mode: "quick" = scan first 50K MFT records (fast), "deep" = scan 500K records (thorough)
fn perform_scan_filesystem(drive_letter: &str, mode: &str) -> recovery_engine::RecoveryScanResult {
    let mut engine = FileSystemRecoveryEngine::new(drive_letter);

    // Build the scan_mode tag that the frontend uses to detect encrypted-drive limitations.
    // Format: "Quick-Encrypted" or "Deep-Encrypted" so the UI can surface a warning that
    // file carving and orphan detection are unavailable on BitLocker volumes.
    let encrypted_mode_name = match mode {
        "deep" => "Deep-Encrypted",
        _      => "Quick-Encrypted",
    };

    // Check admin first
    if !engine.check_admin() {
        return recovery_engine::RecoveryScanResult {
            success: false,
            message: "Administrator privileges required. Please run as Administrator.".to_string(),
            scan_mode: encrypted_mode_name.to_string(),
            drive: drive_letter.to_string(),
            bitlocker_status: Some(engine.check_bitlocker()),
            mft_entries: Vec::new(),
            carved_files: Vec::new(),
            orphan_files: Vec::new(),
            total_files: 0,
            total_recoverable_size: 0,
            scan_duration_ms: 0,
            sectors_scanned: 0,
            mft_records_scanned: 0,
            orphan_records_found: 0,
            requires_admin: true,
        };
    }
    
    // Check BitLocker
    let bl_status = engine.check_bitlocker();
    if bl_status.is_locked {
        return recovery_engine::RecoveryScanResult {
            success: false,
            message: "Drive is BitLocker encrypted and locked. Please unlock with password or recovery key.".to_string(),
            scan_mode: encrypted_mode_name.to_string(),
            drive: drive_letter.to_string(),
            bitlocker_status: Some(bl_status),
            mft_entries: Vec::new(),
            carved_files: Vec::new(),
            orphan_files: Vec::new(),
            total_files: 0,
            total_recoverable_size: 0,
            scan_duration_ms: 0,
            sectors_scanned: 0,
            mft_records_scanned: 0,
            orphan_records_found: 0,
            requires_admin: false,
        };
    }
    
    // Perform filesystem scan with mode-specific parameters.
    // FileSystem API reads through Windows' decryption layer and is much faster
    // than raw disk access, so we can afford higher limits than raw-disk mode.
    // Quick: 250K MFT records  →  ~10-20s, finds user files in common folders.
    // Deep:  500K MFT records  →  thorough, covers most user files on the drive.
    // We cap at 500K to keep JSON output under ~150MB (avoidable OOM / IPC crash).
    let (max_records, hours_limit): (Option<usize>, Option<u64>) = match mode {
        "deep" => (Some(500_000), None),
        _      => (Some(250_000), Some(24)),
    };

    eprintln!("[Main]: {} scan — scanning up to {} MFT records", mode, max_records.unwrap());

    match engine.scan_mft(max_records, hours_limit) {
        Ok(fs_result) => {
            // Convert FileSystemScanResult to RecoveryScanResult
            let mft_entries: Vec<recovery_engine::RecoverableFile> = fs_result.mft_entries.iter().map(|fs_file| {
                recovery_engine::RecoverableFile {
                    id: fs_file.id.clone(),
                    name: fs_file.name.clone(),
                    path: fs_file.path.clone(),
                    size: fs_file.size,
                    extension: fs_file.extension.clone(),
                    category: fs_file.category.clone(),
                    file_type: fs_file.file_type.clone(),
                    modified: fs_file.modified.clone(),
                    created: fs_file.created.clone(),
                    is_deleted: fs_file.is_deleted,
                    recovery_chance: fs_file.recovery_chance,
                    source: fs_file.source.clone(),
                    sector_offset: None,
                    cluster_offset: fs_file.cluster_offset,
                    data_runs: fs_file.data_runs.clone(),
                    fragments: None,
                    partial_recovery: false,
                    recoverable_bytes: fs_file.size,
                    difficulty: "easy".to_string(),
                    age_estimate: "unknown".to_string(),
                }
            }).collect();
            
            recovery_engine::RecoveryScanResult {
                success: true,
                message: format!(
                    "{}. Note: file signature carving and orphan detection are unavailable \
                     on BitLocker-encrypted volumes — only MFT records accessible through \
                     Windows' decryption layer are shown.",
                    fs_result.message
                ),
                scan_mode: encrypted_mode_name.to_string(),
                drive: fs_result.drive,
                bitlocker_status: fs_result.bitlocker_status,
                mft_entries,
                carved_files: Vec::new(), // FileSystem mode doesn't do carving
                orphan_files: Vec::new(), // FileSystem mode doesn't detect orphans
                total_files: fs_result.total_files,
                total_recoverable_size: fs_result.total_recoverable_size,
                scan_duration_ms: fs_result.scan_duration_ms,
                sectors_scanned: 0, // FileSystem mode doesn't scan sectors
                mft_records_scanned: fs_result.mft_records_scanned,
                orphan_records_found: 0,
                requires_admin: true,
            }
        }
        Err(e) => recovery_engine::RecoveryScanResult {
            success: false,
            message: format!("FileSystem scan failed: {}", e),
            scan_mode: encrypted_mode_name.to_string(),
            drive: drive_letter.to_string(),
            bitlocker_status: Some(bl_status),
            mft_entries: Vec::new(),
            carved_files: Vec::new(),
            orphan_files: Vec::new(),
            total_files: 0,
            total_recoverable_size: 0,
            scan_duration_ms: 0,
            sectors_scanned: 0,
            mft_records_scanned: 0,
            orphan_records_found: 0,
            requires_admin: true,
        }
    }
}

fn get_filesystem(drive: &str) -> String {
    #[cfg(windows)]
    {
        let root_path = format!("{}\\", drive);
        let mut wide_path: Vec<u16> = OsStr::new(&root_path)
            .encode_wide()
            .chain(Some(0))
            .collect();
        
        let mut fs_name: Vec<u16> = vec![0; 256];
        
        unsafe {
            let result = winapi::um::fileapi::GetVolumeInformationW(
                wide_path.as_mut_ptr(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                fs_name.as_mut_ptr(),
                fs_name.len() as u32,
            );
            
            if result != 0 {
                let end = fs_name.iter().position(|&c| c == 0).unwrap_or(fs_name.len());
                return String::from_utf16_lossy(&fs_name[..end]);
            }
        }
    }
    
    "Unknown".to_string()
}

fn get_drive_label(drive: &str) -> String {
    #[cfg(windows)]
    {
        let root_path = format!("{}\\", drive);
        let mut wide_path: Vec<u16> = OsStr::new(&root_path)
            .encode_wide()
            .chain(Some(0))
            .collect();
        
        let mut volume_name: Vec<u16> = vec![0; 256];
        
        unsafe {
            let result = winapi::um::fileapi::GetVolumeInformationW(
                wide_path.as_mut_ptr(),
                volume_name.as_mut_ptr(),
                volume_name.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            );
            
            if result != 0 {
                let end = volume_name.iter().position(|&c| c == 0).unwrap_or(volume_name.len());
                let label = String::from_utf16_lossy(&volume_name[..end]);
                if !label.is_empty() {
                    return label;
                }
            }
        }
    }
    
    format!("Local Disk")
}

fn get_drive_space(path: &str) -> (u64, u64) {
    #[cfg(windows)]
    {
        use winapi::um::winnt::ULARGE_INTEGER;
        
        let mut wide_path: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(Some(0))
            .collect();
        
        let mut free_bytes_available = unsafe { std::mem::zeroed::<ULARGE_INTEGER>() };
        let mut total_bytes = unsafe { std::mem::zeroed::<ULARGE_INTEGER>() };
        let mut total_free_bytes = unsafe { std::mem::zeroed::<ULARGE_INTEGER>() };
        
        unsafe {
            let result = winapi::um::fileapi::GetDiskFreeSpaceExW(
                wide_path.as_mut_ptr(),
                &mut free_bytes_available,
                &mut total_bytes,
                &mut total_free_bytes,
            );
            
            if result != 0 {
                return (*total_bytes.QuadPart() as u64, *free_bytes_available.QuadPart() as u64);
            }
        }
    }
    
    (0, 0)
}

fn scan_directory(path: &str) -> ScanResult {
    let mut files = Vec::new();
    let mut total_size = 0u64;
    const MAX_FILES: usize = 10000; // Limit to prevent memory overflow
    let mut file_count = 0usize;
    
    // Walk through directory
    for entry in WalkDir::new(path)
        .max_depth(3) // Reduced depth to prevent scanning too deep
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if file_count >= MAX_FILES {
            eprintln!("Warning: Max file limit reached ({}). Stopping scan.", MAX_FILES);
            break;
        }
        
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                let file_size = metadata.len();
                total_size += file_size;
                file_count += 1;
                
                let path_str = entry.path().to_string_lossy().to_string();
                let name = entry.file_name().to_string_lossy().to_string();
                let extension = entry
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|time| {
                        time.duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|d| d.as_secs())
                    })
                    .map(|secs| {
                        let dt = chrono::DateTime::from_timestamp(secs as i64, 0)
                            .unwrap_or_default();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_else(|| "Unknown".to_string());
                
                files.push(FileInfo {
                    path: path_str,
                    name,
                    size: file_size,
                    extension,
                    modified,
                    is_deleted: false,
                });
            }
        }
    }
    
    ScanResult {
        total_files: files.len(),
        total_size,
        files,
    }
}

fn recover_file_copy(source: &str, destination: &str) -> Result<(), String> {
    // Check if source exists
    if !Path::new(source).exists() {
        return Err(format!("Source file does not exist: {}", source));
    }
    
    // Create destination directory if it doesn't exist
    if let Some(parent) = Path::new(destination).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create destination directory: {}", e))?;
        }
    }
    
    // Copy the file
    match fs::copy(source, destination) {
        Ok(bytes) => {
            eprintln!("Successfully recovered {} bytes", bytes);
            Ok(())
        },
        Err(e) => Err(format!("Failed to recover file: {}", e)),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }
    
    let command = &args[1];
    
    match command.as_str() {
        // Basic Commands
        "drives" => {
            let drives = get_drives();
            let json = serde_json::to_string(&drives).unwrap();
            println!("{}", json);
        }
        "scan" => {
            if args.len() < 3 {
                eprintln!("Usage: data_recovery_backend scan <drive> [mode]");
                eprintln!("  drive: Drive letter (e.g., C)");
                eprintln!("  mode: Optional - 'quick' (last 24h) or 'deep' (all) - default: quick");
                std::process::exit(1);
            }
            let drive = &args[2];
            let mode = args.get(3).map(|s| s.as_str()).unwrap_or("quick");
            
            eprintln!("DEBUG [Main]: Starting scan - drive: {}, mode: {}", drive, mode);
            
            // Use filesystem scanner for deep recovery (works with BitLocker)
            let result = perform_scan_filesystem(drive, mode);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
            
            if !result.success {
                std::process::exit(1);
            }
        }
        "recover" => {
            if args.len() < 4 {
                eprintln!("Usage: data_recovery_backend recover <source> <destination>");
                std::process::exit(1);
            }
            let source = &args[2];
            let destination = &args[3];
            match recover_file_legacy(source, destination) {
                Ok(_) => {
                    println!("{{\"success\": true}}");
                }
                Err(e) => {
                    eprintln!("{{\"success\": false, \"error\": \"{}\"}}", e);
                    std::process::exit(1);
                }
            }
        }
        
        // Admin & BitLocker Commands
        "check-admin" => {
            let status = AdminStatus {
                is_admin: is_admin(),
                message: if is_admin() {
                    "Running with administrator privileges".to_string()
                } else {
                    "Not running as administrator. Please restart with admin privileges.".to_string()
                },
            };
            let json = serde_json::to_string(&status).unwrap();
            println!("{}", json);
        }
        
        "bitlocker-status" => {
            if args.len() < 3 {
                eprintln!("Usage: data_recovery_backend bitlocker-status <drive>");
                std::process::exit(1);
            }
            let drive = &args[2];
            let status = get_bitlocker_status(drive);
            let json = serde_json::to_string(&status).unwrap();
            println!("{}", json);
        }
        
        "bitlocker-unlock-password" => {
            if args.len() < 4 {
                eprintln!("Usage: data_recovery_backend bitlocker-unlock-password <drive> <password>");
                std::process::exit(1);
            }
            let drive = &args[2];
            let password = &args[3];
            let result = unlock_with_password(drive, password);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
            if !result.success {
                std::process::exit(1);
            }
        }
        
        "bitlocker-unlock-key" => {
            if args.len() < 4 {
                eprintln!("Usage: data_recovery_backend bitlocker-unlock-key <drive> <recovery_key>");
                std::process::exit(1);
            }
            let drive = &args[2];
            let key = &args[3];
            let result = unlock_with_recovery_key(drive, key);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
            if !result.success {
                std::process::exit(1);
            }
        }
        
        "bitlocker-lock" => {
            if args.len() < 3 {
                eprintln!("Usage: data_recovery_backend bitlocker-lock <drive>");
                std::process::exit(1);
            }
            let drive = &args[2];
            let result = lock_drive(drive);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
        }
        
        // Professional Recovery Commands
        "deep-scan" => {
            if args.len() < 3 {
                eprintln!("Usage: data_recovery_backend deep-scan <drive> [mode]");
                eprintln!("Modes: quick, deep (default: quick)");
                std::process::exit(1);
            }
            let drive = &args[2];
            let mode = args.get(3).map(|s| s.as_str()).unwrap_or("quick");
            
            // SMART BACKEND ROUTING:
            // Check if drive is encrypted and auto-select backend
            let bl_status = get_bitlocker_status(drive);
            
            if bl_status.is_encrypted && !bl_status.is_locked {
                // Encrypted but unlocked: Use FileSystem mode
                eprintln!("[AUTO-SELECT] BitLocker encrypted drive detected - using FileSystem backend");
                let result = perform_scan_filesystem(drive, mode);
                let json = serde_json::to_string(&result).unwrap();
                println!("{}", json);
                
                if !result.success {
                    std::process::exit(1);
                }
            } else {
                // Not encrypted or locked: Use Raw Disk mode
                eprintln!("[AUTO-SELECT] Unencrypted drive detected - using Raw Disk backend");
                let result = perform_scan(drive, mode);
                let json = serde_json::to_string(&result).unwrap();
                println!("{}", json);
                
                if !result.success {
                    std::process::exit(1);
                }
            }
        }
        
        "recover-deleted" => {
            if args.len() < 5 {
                eprintln!("Usage: data_recovery_backend recover-deleted <drive> <file_json> <destination>");
                std::process::exit(1);
            }
            let drive = &args[2];
            let file_json = &args[3];
            let destination = &args[4];
            
            let result = recover_deleted_file(drive, file_json, destination);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
            
            if !result.success {
                std::process::exit(1);
            }
        }
        
        "file-signatures" => {
            let stats = file_carver::get_signature_stats();
            let json = serde_json::to_string(&stats).unwrap();
            println!("{}", json);
        }
        
        // VSS (Volume Shadow Copy) Commands
        "vss-check" => {
            let available = vss::is_vss_available();
            let result = serde_json::json!({
                "available": available,
                "message": if available {
                    "VSS is available on this system"
                } else {
                    "VSS is not available (Windows only feature)"
                }
            });
            println!("{}", result);
        }
        
        "vss-enumerate" => {
            if args.len() < 3 {
                eprintln!("Usage: data_recovery_backend vss-enumerate <drive>");
                std::process::exit(1);
            }
            let drive = &args[2];
            let result = vss::enumerate_snapshots(drive);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
        }
        
        "vss-list-files" => {
            if args.len() < 3 {
                eprintln!("Usage: data_recovery_backend vss-list-files <snapshot_json> [path]");
                std::process::exit(1);
            }
            let snapshot_json = &args[2];
            let path = args.get(3).map(|s| s.as_str());
            
            let snapshot: vss::VssSnapshot = match serde_json::from_str(snapshot_json) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{{\"success\": false, \"error\": \"Invalid snapshot JSON: {}\"}}", e);
                    std::process::exit(1);
                }
            };
            
            match vss::list_files_in_snapshot(&snapshot, path) {
                Ok(files) => {
                    let result = serde_json::json!({
                        "success": true,
                        "files": files
                    });
                    println!("{}", result);
                }
                Err(e) => {
                    eprintln!("{{\"success\": false, \"error\": \"{}\"}}", e);
                    std::process::exit(1);
                }
            }
        }
        
        "vss-recover" => {
            if args.len() < 5 {
                eprintln!("Usage: data_recovery_backend vss-recover <snapshot_json> <source> <destination>");
                std::process::exit(1);
            }
            let snapshot_json = &args[2];
            let source = &args[3];
            let destination = &args[4];
            
            let snapshot: vss::VssSnapshot = match serde_json::from_str(snapshot_json) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{{\"success\": false, \"error\": \"Invalid snapshot JSON: {}\"}}", e);
                    std::process::exit(1);
                }
            };
            
            match vss::recover_from_snapshot(&snapshot, source, destination) {
                Ok(_) => {
                    println!("{{\"success\": true}}");
                }
                Err(e) => {
                    eprintln!("{{\"success\": false, \"error\": \"{}\"}}", e);
                    std::process::exit(1);
                }
            }
        }
        
        // Help
        "help" | "--help" | "-h" => {
            print_usage();
        }
        
        "version" | "--version" | "-v" => {
            println!("RecoverPro Backend v2.0.0");
            println!("Professional Data Recovery Engine");
            println!("Supports: NTFS MFT parsing, file carving, BitLocker");
        }
        
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn recover_file_legacy(source: &str, destination: &str) -> Result<(), String> {
    if !Path::new(source).exists() {
        return Err(format!("Source file does not exist: {}", source));
    }
    
    if let Some(parent) = Path::new(destination).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create destination directory: {}", e))?;
        }
    }
    
    match fs::copy(source, destination) {
        Ok(bytes) => {
            eprintln!("Successfully recovered {} bytes", bytes);
            Ok(())
        },
        Err(e) => Err(format!("Failed to recover file: {}", e)),
    }
}

fn print_usage() {
    eprintln!("
RecoverPro Backend v2.0.0
================================

BASIC COMMANDS:
  drives                          List all available drives
  scan <path>                     Scan directory for existing files
  recover <source> <destination>  Copy a file (legacy recovery)

ADMIN & BITLOCKER:
  check-admin                     Check if running as administrator
  bitlocker-status <drive>        Check BitLocker status of a drive
  bitlocker-unlock-password <drive> <password>
                                  Unlock BitLocker drive with password
  bitlocker-unlock-key <drive> <recovery_key>
                                  Unlock BitLocker drive with recovery key
  bitlocker-lock <drive>          Lock a BitLocker drive

PROFESSIONAL RECOVERY:
  deep-scan <drive> [mode]        Scan for deleted files
                                  Modes: quick (MFT only), deep (MFT + carving)
  recover-deleted <drive> <file_json> <destination>
                                  Recover a deleted file
  file-signatures                 List supported file signatures

VSS (VOLUME SHADOW COPY):
  vss-check                       Check if VSS is available
  vss-enumerate <drive>           List all snapshots for a drive
  vss-list-files <snapshot_json> [path]
                                  List files in a snapshot
  vss-recover <snapshot_json> <source> <destination>
                                  Recover file from snapshot

OTHER:
  help, --help, -h                Show this help message
  version, --version, -v          Show version information

NOTES:
  - Most recovery commands require Administrator privileges
  - BitLocker encrypted drives must be unlocked before scanning
  - Deep scan mode is more thorough but takes longer
  - VSS requires Windows and existing shadow copies
");
}
