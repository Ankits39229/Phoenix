//! RecoverPro - FileSystem Backend
//! 
//! Optimized for encrypted drives (BitLocker) using file system APIs.
//! Uses Windows decryption layer automatically - no raw disk access.
//! 
//! Requires Administrator privileges for $MFT access.

mod bitlocker;
mod disk_reader;
mod file_carver;
mod filesystem_disk_reader;
mod filesystem_recovery_engine;
mod ntfs_parser;
mod recovery_engine;

use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

use crate::bitlocker::{
    get_bitlocker_status, is_admin, lock_drive, unlock_with_password, unlock_with_recovery_key,
};
use crate::filesystem_recovery_engine::FileSystemRecoveryEngine;
use crate::recovery_engine::RecoveryScanResult;

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
    
    for letter in b'A'..=b'Z' {
        let drive_letter = format!("{}:", letter as char);
        let drive_path = format!("{}\\", drive_letter);
        
        if Path::new(&drive_path).exists() {
            let label = get_drive_label(&drive_letter);
            let (total, free) = get_drive_space(&drive_path);
            let filesystem = get_filesystem(&drive_letter);
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

fn get_drive_label(drive: &str) -> String {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        let drive_path: Vec<u16> = OsStr::new(&format!("{}\\", drive))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut volume_name = [0u16; 261];
        let mut fs_name = [0u16; 261];
        
        unsafe {
            let result = winapi::um::fileapi::GetVolumeInformationW(
                drive_path.as_ptr(),
                volume_name.as_mut_ptr(),
                volume_name.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                fs_name.as_mut_ptr(),
                fs_name.len() as u32,
            );
            
            if result != 0 {
                let len = volume_name.iter().position(|&c| c == 0).unwrap_or(0);
                return String::from_utf16_lossy(&volume_name[..len]);
            }
        }
        
        "Unknown".to_string()
    }
    
    #[cfg(not(windows))]
    {
        "Unknown".to_string()
    }
}

fn get_drive_space(path: &str) -> (u64, u64) {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        let path_wide: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut free_bytes_available: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;
        
        unsafe {
            let result = winapi::um::fileapi::GetDiskFreeSpaceExW(
                path_wide.as_ptr(),
                &mut free_bytes_available as *mut u64 as *mut _,
                &mut total_bytes as *mut u64 as *mut _,
                &mut total_free_bytes as *mut u64 as *mut _,
            );
            
            if result != 0 {
                return (total_bytes, free_bytes_available);
            }
        }
        
        (0, 0)
    }
    
    #[cfg(not(windows))]
    {
        (0, 0)
    }
}

fn get_filesystem(drive: &str) -> String {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        let drive_path: Vec<u16> = OsStr::new(&format!("{}\\", drive))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut fs_name = [0u16; 261];
        
        unsafe {
            let result = winapi::um::fileapi::GetVolumeInformationW(
                drive_path.as_ptr(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                fs_name.as_mut_ptr(),
                fs_name.len() as u32,
            );
            
            if result != 0 {
                let len = fs_name.iter().position(|&c| c == 0).unwrap_or(0);
                return String::from_utf16_lossy(&fs_name[..len]);
            }
        }
        
        "Unknown".to_string()
    }
    
    #[cfg(not(windows))]
    {
        "Unknown".to_string()
    }
}

/// Perform scan using FileSystem backend (for encrypted drives)
/// Mode: "quick" = scan first 50K MFT records (fast), "deep" = scan 500K records (thorough)
fn perform_scan_filesystem(drive_letter: &str, mode: &str) -> RecoveryScanResult {
    let mut engine = FileSystemRecoveryEngine::new(drive_letter);
    
    // Check admin first
    if !engine.check_admin() {
        return RecoveryScanResult {
            success: false,
            message: "Administrator privileges required. Please run as Administrator.".to_string(),
            scan_mode: "FileSystem".to_string(),
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
        return RecoveryScanResult {
            success: false,
            message: "Drive is BitLocker encrypted and locked. Please unlock with password or recovery key.".to_string(),
            scan_mode: "FileSystem".to_string(),
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
    
    // Perform filesystem scan with mode-specific parameters
    // Limit results to prevent massive JSON responses
    let max_records = if mode == "quick" {
        Some(10000)  // Quick scan: 10K records max
    } else {
        Some(50000)  // Deep scan: 50K records max
    };
    let hours_limit = if mode == "quick" { Some(24) } else { None };
    
    eprintln!("DEBUG [MainFS]: {} MODE - scanning up to {} MFT records", 
        mode.to_uppercase(), max_records.unwrap());
    
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
            
            RecoveryScanResult {
                success: true,
                message: format!("{} (FileSystem Mode - Decrypted)", fs_result.message),
                scan_mode: "FileSystem".to_string(),
                drive: fs_result.drive,
                bitlocker_status: fs_result.bitlocker_status,
                mft_entries,
                carved_files: Vec::new(),
                orphan_files: Vec::new(),
                total_files: fs_result.total_files,
                total_recoverable_size: fs_result.total_recoverable_size,
                scan_duration_ms: fs_result.scan_duration_ms,
                sectors_scanned: 0,
                mft_records_scanned: fs_result.mft_records_scanned,
                orphan_records_found: 0,
                requires_admin: true,
            }
        }
        Err(e) => RecoveryScanResult {
            success: false,
            message: format!("FileSystem scan failed: {}", e),
            scan_mode: "FileSystem".to_string(),
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

/// File info structure for recovery
#[derive(Serialize, Deserialize, Debug)]
struct FileInfoForRecovery {
    id: Option<String>,
    name: String,
    path: String,
    size: u64,
    extension: Option<String>,
    category: Option<String>,
    file_type: Option<String>,
    modified: Option<String>,
    created: Option<String>,
    is_deleted: Option<bool>,
    recovery_chance: Option<u8>,
    source: Option<String>,
    cluster_offset: Option<i64>,
    data_runs: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DataRun {
    cluster_offset: i64,
    cluster_count: i64,
}

/// File signature patterns for carving
struct FileSignature {
    extension: &'static str,
    header: &'static [u8],
    footer: Option<&'static [u8]>,
    max_size: u64,  // Maximum expected file size in bytes
}

fn get_carving_signatures() -> Vec<FileSignature> {
    vec![
        FileSignature { extension: "pdf", header: b"%PDF-", footer: Some(b"%%EOF"), max_size: 500 * 1024 * 1024 },
        FileSignature { extension: "jpg", header: &[0xFF, 0xD8, 0xFF], footer: Some(&[0xFF, 0xD9]), max_size: 100 * 1024 * 1024 },
        FileSignature { extension: "png", header: &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A], footer: Some(&[0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82]), max_size: 100 * 1024 * 1024 },
        FileSignature { extension: "zip", header: &[0x50, 0x4B, 0x03, 0x04], footer: None, max_size: 500 * 1024 * 1024 },
        FileSignature { extension: "docx", header: &[0x50, 0x4B, 0x03, 0x04], footer: None, max_size: 100 * 1024 * 1024 },
        FileSignature { extension: "xlsx", header: &[0x50, 0x4B, 0x03, 0x04], footer: None, max_size: 100 * 1024 * 1024 },
        FileSignature { extension: "mp3", header: &[0x49, 0x44, 0x33], footer: None, max_size: 50 * 1024 * 1024 },
        FileSignature { extension: "mp4", header: b"ftyp", footer: None, max_size: 2 * 1024 * 1024 * 1024 },
    ]
}

/// Carve a file from raw volume by scanning for file signatures
/// This works through BitLocker because we use the volume handle (\\.\C:)
/// which Windows decrypts automatically.
/// Uses keyword matching from the filename to identify the correct file
/// among potentially many matches on the volume.
fn carve_file_from_volume(drive: &str, file_info: &FileInfoForRecovery, destination: &str) -> RecoveryResult {
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    
    let extension = file_info.extension.as_deref().unwrap_or("").to_lowercase();
    
    // Find matching signature
    let signatures = get_carving_signatures();
    let sig = match signatures.iter().find(|s| s.extension == extension) {
        Some(s) => s,
        None => {
            return RecoveryResult {
                success: false,
                message: format!("File carving not supported for .{} files", extension),
                bytes_recovered: 0,
                source_path: file_info.path.clone(),
                destination_path: destination.to_string(),
            };
        }
    };
    
    // Extract keywords from filename for content verification
    let name_without_ext = file_info.name.rsplit('.').skip(1).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(".");
    let keywords: Vec<String> = name_without_ext
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_lowercase())
        .collect();
    
    eprintln!("[Carving] Starting file carving for '{}' (.{}) on drive {}", 
        file_info.name, extension, drive);
    eprintln!("[Carving] Keywords for matching: {:?}", keywords);
    
    let drive_letter = drive.trim_end_matches('\\').trim_end_matches(':');
    
    // Open volume handle for decrypted reading
    match crate::filesystem_disk_reader::FileSystemDiskReader::new(drive_letter) {
        Ok(mut reader) => {
            let cluster_size = 4096u64;
            let chunk_clusters = 1024u64; // 4MB chunks
            let chunk_size = chunk_clusters * cluster_size;
            let max_scan_bytes: u64 = 8 * 1024 * 1024 * 1024; // Scan up to 8GB
            // Use the known original file size (with tolerance) to constrain the carve.
            // Without this, the carver reads hundreds of MB of random data until it stumbles
            // upon a footer that belongs to a DIFFERENT file, producing corrupt output.
            let max_file_size = if file_info.size > 0 {
                // Allow +20% tolerance (clusters are slightly larger than real size)
                let constrained = (file_info.size as f64 * 1.2) as u64;
                eprintln!("[Carving] Constraining max carved size to {} bytes (original size {} + 20%%)",
                    constrained, file_info.size);
                constrained
            } else {
                sig.max_size
            };
            let min_file_size: u64 = 1024; // Minimum 1KB to be a valid file
            
            let mut scan_offset: u64 = 0;
            let mut scanned_bytes: u64 = 0;
            let mut candidates_found: u32 = 0;
            let mut best_match: Option<Vec<u8>> = None;
            let mut best_keyword_score: usize = 0;
            let max_candidates = 50; // Don't check more than 50 matches
            
            // For boundary detection: keep last few bytes of previous chunk
            let overlap_size = sig.header.len().max(8);
            let mut prev_tail: Vec<u8> = Vec::new();
            
            eprintln!("[Carving] Scanning volume for {} header signature: {:?}", extension, sig.header);
            
            while scanned_bytes < max_scan_bytes && candidates_found < max_candidates {
                let cluster_offset = scan_offset / cluster_size;
                
                match reader.read_clusters(cluster_offset, chunk_clusters, cluster_size) {
                    Ok(data) => {
                        // Combine overlap from previous chunk for boundary detection
                        let search_buf = if !prev_tail.is_empty() {
                            let mut combined = prev_tail.clone();
                            combined.extend_from_slice(&data);
                            combined
                        } else {
                            data.clone()
                        };
                        let offset_adjustment = if !prev_tail.is_empty() { prev_tail.len() } else { 0 };
                        
                        // Search for all header signatures in this chunk
                        let header = sig.header;
                        let mut search_pos = 0;
                        
                        while search_pos < search_buf.len().saturating_sub(header.len()) {
                            let found = if extension == "mp4" {
                                // MP4: ftyp at offset 4 from box start
                                search_pos >= 4 && &search_buf[search_pos..search_pos + header.len()] == header
                            } else {
                                &search_buf[search_pos..search_pos + header.len()] == header
                            };
                            
                            if !found {
                                search_pos += 1;
                                continue;
                            }
                            
                            candidates_found += 1;
                            let file_start_in_buf = if extension == "mp4" { search_pos - 4 } else { search_pos };
                            let abs_offset = scan_offset + file_start_in_buf as u64 - offset_adjustment as u64;
                            
                            eprintln!("[Carving] Found {} header #{} at byte offset {} ({} MB)", 
                                extension, candidates_found, abs_offset, abs_offset / (1024 * 1024));
                            
                            // Read the complete file from this position
                            let carved = read_carved_file(
                                &mut reader, abs_offset, cluster_size, chunk_clusters, 
                                max_file_size, sig.footer
                            );
                            
                            if let Some(file_data) = carved {
                                let file_size = file_data.len() as u64;
                                
                                // Skip tiny files (likely corrupted)
                                if file_size < min_file_size {
                                    eprintln!("[Carving]   Skipping: too small ({} bytes)", file_size);
                                    search_pos += 1;
                                    continue;
                                }
                                
                                eprintln!("[Carving]   Carved file size: {} bytes ({} KB)", file_size, file_size / 1024);
                                
                                // Validate the carved file's internal structure.
                                // File carving reads CONTIGUOUS sectors, but deleted files are often
                                // fragmented across the disk. This validation catches the common case
                                // where the carved data is: real-header + garbage-from-other-files.
                                if !validate_carved_file(&file_data, &extension) {
                                    eprintln!("[Carving]   Skipping: FAILED structural validation (likely fragmented/corrupt)");
                                    search_pos += 1;
                                    continue;
                                }
                                
                                // Check keyword match score
                                if !keywords.is_empty() {
                                    let score = count_keyword_matches(&file_data, &keywords);
                                    eprintln!("[Carving]   Keyword score: {}/{}", score, keywords.len());
                                    
                                    if score > best_keyword_score {
                                        best_keyword_score = score;
                                        best_match = Some(file_data);
                                        eprintln!("[Carving]   New best match! (score: {})", score);
                                        
                                        // Perfect match — all keywords found
                                        if score == keywords.len() {
                                            eprintln!("[Carving]   Perfect keyword match! Stopping scan.");
                                            break;
                                        }
                                    }
                                } else {
                                    // No keywords to match — use first valid file > 1KB
                                    if best_match.is_none() {
                                        best_match = Some(file_data);
                                    }
                                }
                            }
                            
                            search_pos += 1;
                            
                            if candidates_found >= max_candidates {
                                break;
                            }
                        }
                        
                        // Check if we found a perfect match
                        if best_keyword_score == keywords.len() && !keywords.is_empty() {
                            break;
                        }
                        
                        // Save tail for boundary overlap
                        if data.len() > overlap_size {
                            prev_tail = data[data.len() - overlap_size..].to_vec();
                        }
                        
                        scan_offset += data.len() as u64;
                        scanned_bytes += data.len() as u64;
                        
                        // Progress logging every 512MB
                        if scanned_bytes % (512 * 1024 * 1024) < chunk_size {
                            eprintln!("[Carving] Scanned {} MB, {} candidates found so far...", 
                                scanned_bytes / (1024 * 1024), candidates_found);
                        }
                    }
                    Err(e) => {
                        // Skip unreadable areas
                        scan_offset += chunk_size;
                        scanned_bytes += chunk_size;
                    }
                }
            }
            
            eprintln!("[Carving] Scan complete. Scanned {} MB, found {} candidates, best keyword score: {}/{}",
                scanned_bytes / (1024 * 1024), candidates_found, best_keyword_score, keywords.len());
            
            // Write the best match
            if let Some(file_data) = best_match {
                let dest_path = Path::new(destination);
                if let Some(parent) = dest_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                
                match fs::File::create(dest_path) {
                    Ok(mut file) => {
                        match file.write_all(&file_data) {
                            Ok(_) => {
                                let msg = if best_keyword_score > 0 {
                                    format!("Recovered {} bytes via file carving (keyword match: {}/{})", 
                                        file_data.len(), best_keyword_score, keywords.len())
                                } else {
                                    format!("Recovered {} bytes via file carving (signature-based recovery)", 
                                        file_data.len())
                                };
                                RecoveryResult {
                                    success: true,
                                    message: msg,
                                    bytes_recovered: file_data.len() as u64,
                                    source_path: file_info.path.clone(),
                                    destination_path: destination.to_string(),
                                }
                            }
                            Err(e) => RecoveryResult {
                                success: false,
                                message: format!("Failed to write carved file: {}", e),
                                bytes_recovered: 0,
                                source_path: file_info.path.clone(),
                                destination_path: destination.to_string(),
                            },
                        }
                    }
                    Err(e) => RecoveryResult {
                        success: false,
                        message: format!("Failed to create output file: {}", e),
                        bytes_recovered: 0,
                        source_path: file_info.path.clone(),
                        destination_path: destination.to_string(),
                    },
                }
            } else {
                RecoveryResult {
                    success: false,
                    message: format!("File carving could not find .{} file matching '{}' on the volume ({} MB scanned, {} candidates checked). Data may have been overwritten.", 
                        extension, file_info.name, scanned_bytes / (1024 * 1024), candidates_found),
                    bytes_recovered: 0,
                    source_path: file_info.path.clone(),
                    destination_path: destination.to_string(),
                }
            }
        }
        Err(e) => RecoveryResult {
            success: false,
            message: format!("Failed to open drive for carving: {}", e),
            bytes_recovered: 0,
            source_path: file_info.path.clone(),
            destination_path: destination.to_string(),
        },
    }
}

/// Read a complete carved file starting from a given byte offset
fn read_carved_file(
    reader: &mut crate::filesystem_disk_reader::FileSystemDiskReader,
    start_offset: u64,
    cluster_size: u64,
    chunk_clusters: u64,
    max_size: u64,
    footer: Option<&[u8]>,
) -> Option<Vec<u8>> {
    let chunk_size = chunk_clusters * cluster_size;
    let start_cluster = start_offset / cluster_size;
    let byte_offset_in_cluster = (start_offset % cluster_size) as usize;
    
    // Read first chunk
    let first_data = reader.read_clusters(start_cluster, chunk_clusters, cluster_size).ok()?;
    if byte_offset_in_cluster >= first_data.len() {
        return None;
    }
    
    let mut file_data = Vec::with_capacity(max_size.min(50 * 1024 * 1024) as usize);
    file_data.extend_from_slice(&first_data[byte_offset_in_cluster..]);
    
    // Check if footer is in first chunk
    if let Some(footer_bytes) = footer {
        if let Some(pos) = find_footer(&file_data, footer_bytes) {
            file_data.truncate(pos + footer_bytes.len());
            return Some(file_data);
        }
    }
    
    // Read more chunks until footer or max size
    let mut read_offset = start_offset + first_data.len() as u64 - byte_offset_in_cluster as u64;
    // For large reads, align to cluster boundary
    let next_cluster = (read_offset + cluster_size - 1) / cluster_size;
    read_offset = next_cluster * cluster_size;
    
    while file_data.len() < max_size as usize {
        let read_cluster = read_offset / cluster_size;
        match reader.read_clusters(read_cluster, chunk_clusters, cluster_size) {
            Ok(next_data) => {
                let prev_len = file_data.len();
                file_data.extend_from_slice(&next_data);
                read_offset += next_data.len() as u64;
                
                // Check for footer in newly added data
                if let Some(footer_bytes) = footer {
                    let search_start = prev_len.saturating_sub(footer_bytes.len());
                    if let Some(pos) = find_footer_from(&file_data, footer_bytes, search_start) {
                        file_data.truncate(pos + footer_bytes.len());
                        return Some(file_data);
                    }
                }
                
                // No footer type — use max size limit
                if footer.is_none() && file_data.len() >= max_size as usize {
                    return Some(file_data);
                }
            }
            Err(_) => break,
        }
    }
    
    // If we have footer type but didn't find it, the file might be corrupted
    // Return what we have if it's reasonably sized
    if footer.is_some() && file_data.len() > 1024 {
        Some(file_data)
    } else if footer.is_none() {
        Some(file_data)
    } else {
        None
    }
}

/// Validate that a carved file has valid internal structure.
/// Returns true if the data looks like a plausible file of its type.
/// This catches the common carving failure where contiguous sector reads
/// grab data from DIFFERENT files (fragmented on disk) and produce garbage.
fn validate_carved_file(data: &[u8], extension: &str) -> bool {
    match extension.to_lowercase().as_str() {
        "pdf" => validate_carved_pdf(data),
        "docx" | "xlsx" | "pptx" | "zip" | "jar" => validate_carved_zip(data),
        "png" => validate_carved_png(data),
        "jpg" | "jpeg" => validate_carved_jpeg(data),
        _ => true, // No validation available — accept
    }
}

/// Validate carved PDF structure.
/// A well-formed PDF has: %PDF-x.y header, cross-ref table, startxref pointer, %%EOF.
/// If the carved data is a mix of the real PDF + random clusters from other files,
/// the cross-reference table will be missing or point to garbage.
fn validate_carved_pdf(data: &[u8]) -> bool {
    if data.len() < 100 {
        return false;
    }
    // Must start with %PDF-
    if &data[0..5] != b"%PDF-" {
        return false;
    }
    // Check the last 2KB for PDF trailer markers
    let tail_size = data.len().min(2048);
    let tail = &data[data.len() - tail_size..];
    let tail_str = String::from_utf8_lossy(tail);

    // Must have %%EOF in the tail
    if !tail_str.contains("%%EOF") {
        eprintln!("[Carving] PDF validation FAILED: no %%EOF in last {}B", tail_size);
        return false;
    }

    // Must have startxref before %%EOF — this is the cross-reference pointer.
    // Without it, no PDF reader can locate the file's object catalog.
    if !tail_str.contains("startxref") {
        eprintln!("[Carving] PDF validation FAILED: no 'startxref' in last {}B (likely fragmented/corrupt)", tail_size);
        return false;
    }

    // Validate that the startxref value points within the file (sanity check)
    if let Some(pos) = tail_str.rfind("startxref") {
        let after_startxref = &tail_str[pos + 9..];
        // The offset should be a number on the next line
        let offset_str = after_startxref
            .trim_start_matches(|c: char| c == '\r' || c == '\n' || c == ' ')
            .lines()
            .next()
            .unwrap_or("")
            .trim();
        if let Ok(xref_offset) = offset_str.parse::<u64>() {
            if xref_offset >= data.len() as u64 {
                eprintln!("[Carving] PDF validation FAILED: startxref offset {} >= file size {} (corrupt cross-reference)",
                    xref_offset, data.len());
                return false;
            }
            // Check that something resembling xref/trailer exists at that offset
            if xref_offset < data.len() as u64 {
                let xref_pos = xref_offset as usize;
                let check_len = data.len().min(xref_pos + 20) - xref_pos;
                let at_xref = String::from_utf8_lossy(&data[xref_pos..xref_pos + check_len]);
                if !at_xref.starts_with("xref") && !at_xref.contains("obj") && !at_xref.contains("XRef") {
                    eprintln!("[Carving] PDF validation FAILED: no xref/obj at startxref offset {} (found: {:?})",
                        xref_offset, &at_xref[..at_xref.len().min(30)]);
                    return false;
                }
            }
        }
    }

    // Also check for at least some PDF objects in the body (obj/endobj pairs)
    let first_64k = &data[..data.len().min(65536)];
    let body_str = String::from_utf8_lossy(first_64k);
    let obj_count = body_str.matches(" obj").count();
    if obj_count < 2 {
        eprintln!("[Carving] PDF validation FAILED: only {} obj markers in first 64KB (expected >= 2)", obj_count);
        return false;
    }

    eprintln!("[Carving] PDF validation PASSED: header OK, startxref OK, {} objects found", obj_count);
    true
}

/// Validate carved ZIP-based files (docx, xlsx, pptx, zip, jar)
fn validate_carved_zip(data: &[u8]) -> bool {
    // ZIP files start with PK\x03\x04
    if data.len() < 30 || &data[0..4] != b"PK\x03\x04" {
        return false;
    }
    // Check for End of Central Directory Record (PK\x05\x06) in last 256 bytes
    let tail_size = data.len().min(256);
    let tail = &data[data.len() - tail_size..];
    for i in 0..tail.len().saturating_sub(3) {
        if &tail[i..i + 4] == b"PK\x05\x06" {
            return true;
        }
    }
    eprintln!("[Carving] ZIP validation FAILED: no End of Central Directory record");
    false
}

/// Validate carved PNG
fn validate_carved_png(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }
    // PNG must end with IEND chunk
    let tail_size = data.len().min(32);
    let tail = &data[data.len() - tail_size..];
    for i in 0..tail.len().saturating_sub(3) {
        if &tail[i..i + 4] == b"IEND" {
            return true;
        }
    }
    eprintln!("[Carving] PNG validation FAILED: no IEND chunk at end");
    false
}

/// Validate carved JPEG
fn validate_carved_jpeg(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    // JPEG must start with FF D8 FF and end with FF D9
    if data[0] != 0xFF || data[1] != 0xD8 || data[2] != 0xFF {
        return false;
    }
    // Check last 2 bytes for EOI marker
    if data[data.len() - 2] != 0xFF || data[data.len() - 1] != 0xD9 {
        eprintln!("[Carving] JPEG validation FAILED: no EOI marker (FF D9) at end");
        return false;
    }
    true
}

/// Count how many keywords from the filename appear in the carved file data
fn count_keyword_matches(data: &[u8], keywords: &[String]) -> usize {
    // Convert data to lowercase string for searching
    // Only check first 100KB for performance (metadata/title pages are at the start)
    let check_size = data.len().min(100 * 1024);
    let text = String::from_utf8_lossy(&data[..check_size]).to_lowercase();
    
    keywords.iter()
        .filter(|kw| text.contains(kw.as_str()))
        .count()
}

/// Find a byte pattern (footer) in a buffer
fn find_footer(data: &[u8], footer: &[u8]) -> Option<usize> {
    find_footer_from(data, footer, 0)
}

/// Find a byte pattern (footer) in a buffer starting from a given position
/// Searches backwards from end for efficiency with large files
fn find_footer_from(data: &[u8], footer: &[u8], start: usize) -> Option<usize> {
    if data.len() < footer.len() || start >= data.len() {
        return None;
    }
    
    // Search from end backwards (footer is typically at/near end)
    let search_end = data.len() - footer.len();
    let search_start = start;
    
    // Search last 1MB first
    let quick_start = if search_end > 1024 * 1024 { search_end - 1024 * 1024 } else { search_start };
    for i in (quick_start..=search_end).rev() {
        if &data[i..i + footer.len()] == footer {
            return Some(i);
        }
    }
    
    // If not found in last 1MB, search remaining
    if quick_start > search_start {
        for i in (search_start..quick_start).rev() {
            if &data[i..i + footer.len()] == footer {
                return Some(i);
            }
        }
    }
    
    None
}

/// Recover a resident file (data stored in MFT record itself)
fn recover_resident_file(drive: &str, file_info: &FileInfoForRecovery, destination: &str) -> RecoveryResult {
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    
    eprintln!("[FileSystem] Recovering resident file: {}", file_info.name);
    
    // Extract MFT record number from id (format: fs_mft_12345 or usn_mft_12345)
    let record_num = if let Some(id) = &file_info.id {
        if let Some(num_str) = id.strip_prefix("fs_mft_") {
            num_str.parse::<u64>().ok()
        } else if let Some(num_str) = id.strip_prefix("usn_mft_") {
            num_str.parse::<u64>().ok()
        } else {
            None
        }
    } else {
        None
    };
    
    let record_num = match record_num {
        Some(num) => num,
        None => {
            return RecoveryResult {
                success: false,
                message: "Cannot extract MFT record number from file ID".to_string(),
                bytes_recovered: 0,
                source_path: file_info.path.clone(),
                destination_path: destination.to_string(),
            };
        }
    };
    
    let drive_letter = drive.trim_end_matches('\\').trim_end_matches(':');
    
    // Read the MFT record
    match crate::filesystem_disk_reader::FileSystemDiskReader::new(drive_letter) {
        Ok(mut reader) => {
            match reader.read_mft_record(record_num) {
                Ok(record_data) => {
                    // First, verify the MFT record still belongs to our file
                    // (not reused by another file)
                    if let Some(record_filename) = extract_filename_from_record(&record_data) {
                        if record_filename.to_lowercase() != file_info.name.to_lowercase() {
                            eprintln!("[FileSystem] MFT record {} reused by '{}', expected '{}' — skipping resident recovery",
                                record_num, record_filename, file_info.name);
                            return RecoveryResult {
                                success: false,
                                message: format!("MFT record reused by '{}', resident data belongs to wrong file", record_filename),
                                bytes_recovered: 0,
                                source_path: file_info.path.clone(),
                                destination_path: destination.to_string(),
                            };
                        }
                    }
                    
                    // Extract resident data from the DATA attribute
                    if let Some(resident_data) = extract_resident_data(&record_data) {
                        if resident_data.is_empty() {
                            return RecoveryResult {
                                success: false,
                                message: "Resident data is empty".to_string(),
                                bytes_recovered: 0,
                                source_path: file_info.path.clone(),
                                destination_path: destination.to_string(),
                            };
                        }
                        
                        eprintln!("[FileSystem] Extracted {} bytes of resident data", resident_data.len());
                        
                        // Write to destination
                        let dest_path = Path::new(destination);
                        if let Some(parent) = dest_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        
                        match fs::File::create(dest_path) {
                            Ok(mut file) => {
                                match file.write_all(&resident_data) {
                                    Ok(_) => {
                                        return RecoveryResult {
                                            success: true,
                                            message: format!("Recovered {} bytes from resident MFT data", resident_data.len()),
                                            bytes_recovered: resident_data.len() as u64,
                                            source_path: file_info.path.clone(),
                                            destination_path: destination.to_string(),
                                        };
                                    }
                                    Err(e) => {
                                        return RecoveryResult {
                                            success: false,
                                            message: format!("Failed to write resident data: {}", e),
                                            bytes_recovered: 0,
                                            source_path: file_info.path.clone(),
                                            destination_path: destination.to_string(),
                                        };
                                    }
                                }
                            }
                            Err(e) => {
                                return RecoveryResult {
                                    success: false,
                                    message: format!("Failed to create output file: {}", e),
                                    bytes_recovered: 0,
                                    source_path: file_info.path.clone(),
                                    destination_path: destination.to_string(),
                                };
                            }
                        }
                    } else {
                        return RecoveryResult {
                            success: false,
                            message: "No resident data found in MFT record".to_string(),
                            bytes_recovered: 0,
                            source_path: file_info.path.clone(),
                            destination_path: destination.to_string(),
                        };
                    }
                }
                Err(e) => {
                    return RecoveryResult {
                        success: false,
                        message: format!("Failed to read MFT record {}: {}", record_num, e),
                        bytes_recovered: 0,
                        source_path: file_info.path.clone(),
                        destination_path: destination.to_string(),
                    };
                }
            }
        }
        Err(e) => {
            return RecoveryResult {
                success: false,
                message: format!("Failed to open drive: {}", e),
                bytes_recovered: 0,
                source_path: file_info.path.clone(),
                destination_path: destination.to_string(),
            };
        }
    }
}

/// Extract resident data from a DATA attribute in an MFT record
/// Extract the primary filename from an MFT record
/// Used to detect if a record has been reused by checking if the name matches
fn extract_filename_from_record(record: &[u8]) -> Option<String> {
    const MFT_SIGNATURE: &[u8] = b"FILE";
    const ATTRIBUTE_FILENAME: u32 = 0x30;
    const ATTRIBUTE_END: u32 = 0xFFFFFFFF;
    
    if record.len() < 56 || &record[0..4] != MFT_SIGNATURE {
        return None;
    }
    
    let attr_offset_start = u16::from_le_bytes([record[20], record[21]]) as usize;
    if attr_offset_start >= record.len() {
        return None;
    }
    
    let mut offset = attr_offset_start;
    let mut best_name: Option<String> = None;
    
    while offset + 16 <= record.len() {
        let attr_type = u32::from_le_bytes([
            record[offset], record[offset + 1],
            record[offset + 2], record[offset + 3],
        ]);
        
        if attr_type == ATTRIBUTE_END {
            break;
        }
        
        let attr_length = u32::from_le_bytes([
            record[offset + 4], record[offset + 5],
            record[offset + 6], record[offset + 7],
        ]) as usize;
        
        if attr_length == 0 || offset + attr_length > record.len() {
            break;
        }
        
        if attr_type == ATTRIBUTE_FILENAME {
            let non_resident = record[offset + 8];
            if non_resident == 0 {
                let content_offset = u16::from_le_bytes([
                    record[offset + 20], record[offset + 21],
                ]) as usize;
                
                let fn_start = offset + content_offset;
                // Filename attribute structure:
                // +64: filename namespace (byte)
                // +66: filename length in chars (byte at fn_start + 64)
                // Actually: parent ref(8) + creation(8) + modified(8) + mft_modified(8) + read(8) + alloc_size(8) + real_size(8) + flags(4) + reparse(4) = 66 bytes
                // +64: name_length (1 byte)
                // +65: namespace (1 byte)
                // +66: name (UTF-16LE)
                if fn_start + 66 < record.len() {
                    let name_len = record[fn_start + 64] as usize;
                    let namespace = record[fn_start + 65];
                    let name_start = fn_start + 66;
                    let name_end = name_start + name_len * 2;
                    
                    if name_end <= record.len() {
                        let name: String = (name_start..name_end)
                            .step_by(2)
                            .map(|i| {
                                u16::from_le_bytes([record[i], record[i + 1]])
                            })
                            .collect::<Vec<u16>>()
                            .iter()
                            .map(|&c| char::from_u32(c as u32).unwrap_or('?'))
                            .collect();
                        
                        // Prefer Win32 namespace (0x01 or 0x03) over DOS (0x02)
                        if namespace != 0x02 {
                            return Some(name);
                        }
                        if best_name.is_none() {
                            best_name = Some(name);
                        }
                    }
                }
            }
        }
        
        offset += attr_length;
    }
    
    best_name
}

fn extract_resident_data(record: &[u8]) -> Option<Vec<u8>> {
    const MFT_SIGNATURE: &[u8] = b"FILE";
    const ATTRIBUTE_DATA: u32 = 0x80;
    const ATTRIBUTE_END: u32 = 0xFFFFFFFF;
    
    if record.len() < 1024 || &record[0..4] != MFT_SIGNATURE {
        return None;
    }
    
    // Get attributes offset
    let attr_offset_start = u16::from_le_bytes([record[20], record[21]]) as usize;
    if attr_offset_start >= record.len() {
        return None;
    }
    
    let mut offset = attr_offset_start;
    
    // Scan for DATA attribute
    while offset + 16 <= record.len() {
        let attr_type = u32::from_le_bytes([
            record[offset],
            record[offset + 1],
            record[offset + 2],
            record[offset + 3],
        ]);
        
        if attr_type == ATTRIBUTE_END {
            break;
        }
        
        let attr_length = u32::from_le_bytes([
            record[offset + 4],
            record[offset + 5],
            record[offset + 6],
            record[offset + 7],
        ]) as usize;
        
        if attr_length == 0 || offset + attr_length > record.len() {
            break;
        }
        
        if attr_type == ATTRIBUTE_DATA {
            // Check if resident (byte at offset 8 from attr start)
            let non_resident = record[offset + 8];
            
            if non_resident == 0 {
                // Resident - extract data
                if offset + 24 <= record.len() {
                    let content_length = u32::from_le_bytes([
                        record[offset + 16],
                        record[offset + 17],
                        record[offset + 18],
                        record[offset + 19],
                    ]) as usize;
                    
                    let content_offset = u16::from_le_bytes([
                        record[offset + 20],
                        record[offset + 21],
                    ]) as usize;
                    
                    let data_start = offset + content_offset;
                    let data_end = data_start + content_length;
                    
                    if data_end <= record.len() {
                        return Some(record[data_start..data_end].to_vec());
                    }
                }
            }
            
            // Found DATA attribute but not resident or invalid
            return None;
        }
        
        offset += attr_length;
    }
    
    None
}

/// Recover a deleted file from the Windows Recycle Bin
/// Windows stores deleted files as $R{hash}.{ext} (data) + $I{hash}.{ext} (metadata)
/// The $I file contains: header(8 bytes) + file_size(8 bytes) + deletion_time(8 bytes) + original_path(520+ bytes)
fn recover_from_recycle_bin(drive: &str, file_info: &FileInfoForRecovery, destination: &str) -> RecoveryResult {
    use std::fs;
    use std::path::Path;
    
    let drive_letter = drive.trim_end_matches('\\').trim_end_matches(':').to_uppercase();
    let recycle_bin_path = format!("{}:\\$Recycle.Bin", drive_letter);
    
    eprintln!("[RecycleBin] Searching in: {}", recycle_bin_path);
    
    let recycle_dir = Path::new(&recycle_bin_path);
    if !recycle_dir.exists() {
        return RecoveryResult {
            success: false,
            message: "Recycle Bin directory not found".to_string(),
            bytes_recovered: 0,
            source_path: file_info.path.clone(),
            destination_path: destination.to_string(),
        };
    }
    
    // Iterate through SID subdirectories in $Recycle.Bin
    let sid_dirs = match fs::read_dir(recycle_dir) {
        Ok(dirs) => dirs,
        Err(e) => {
            eprintln!("[RecycleBin] Cannot read Recycle Bin directory: {}", e);
            return RecoveryResult {
                success: false,
                message: format!("Cannot read Recycle Bin: {}", e),
                bytes_recovered: 0,
                source_path: file_info.path.clone(),
                destination_path: destination.to_string(),
            };
        }
    };
    
    let target_name = file_info.name.to_lowercase();
    let target_ext = if let Some(pos) = file_info.name.rfind('.') {
        file_info.name[pos..].to_lowercase()
    } else {
        String::new()
    };
    
    let mut sid_count = 0;
    let mut i_files_scanned = 0;
    
    for sid_entry in sid_dirs {
        let sid_dir = match sid_entry {
            Ok(e) => e.path(),
            Err(e) => {
                eprintln!("[RecycleBin] Failed to read SID entry: {}", e);
                continue;
            }
        };
        
        if !sid_dir.is_dir() {
            continue;
        }
        
        sid_count += 1;
        eprintln!("[RecycleBin] Scanning SID dir: {}", sid_dir.display());
        
        // Search for $I files with matching extension
        let entries = match fs::read_dir(&sid_dir) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("[RecycleBin] Cannot read SID dir: {}", e);
                continue;
            }
        };
        
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            
            let fname = entry.file_name().to_string_lossy().to_string();
            
            // Look for $I files (metadata files)
            if !fname.starts_with("$I") {
                continue;
            }
            
            i_files_scanned += 1;
            
            // Check extension matches
            if !target_ext.is_empty() {
                let i_ext = if let Some(pos) = fname.rfind('.') {
                    fname[pos..].to_lowercase()
                } else {
                    String::new()
                };
                if i_ext != target_ext {
                    continue;
                }
            }
            
            // Read the $I file to check if it matches our target
            let i_path = entry.path();
            let i_data = match fs::read(&i_path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            
            // $I file format (version 2, Windows 10+):
            // Bytes 0-7: Version header (01 00 00 00 00 00 00 00 = v1, 02 00 = v2)
            // Bytes 8-15: File size (u64 LE)
            // Bytes 16-23: Deletion timestamp (FILETIME, u64 LE)
            // Bytes 24-27: Path length in chars (v2 only, u32 LE)
            // Bytes 28+: Original file path (UTF-16LE)
            // For v1: path starts at offset 24, fixed 520 bytes
            
            if i_data.len() < 28 {
                continue;
            }
            
            let version = u64::from_le_bytes([
                i_data[0], i_data[1], i_data[2], i_data[3],
                i_data[4], i_data[5], i_data[6], i_data[7],
            ]);
            
            let original_path = if version == 2 {
                // Version 2: path length at offset 24, path at offset 28
                let path_len = u32::from_le_bytes([
                    i_data[24], i_data[25], i_data[26], i_data[27],
                ]) as usize;
                let path_bytes_end = 28 + path_len * 2;
                if path_bytes_end <= i_data.len() {
                    let utf16: Vec<u16> = (28..path_bytes_end)
                        .step_by(2)
                        .map(|i| u16::from_le_bytes([i_data[i], i_data[i + 1]]))
                        .collect();
                    String::from_utf16_lossy(&utf16).trim_end_matches('\0').to_string()
                } else {
                    continue;
                }
            } else {
                // Version 1: path at offset 24, up to 520 bytes (260 UTF-16 chars)
                let path_end = (24 + 520).min(i_data.len());
                let utf16: Vec<u16> = (24..path_end)
                    .step_by(2)
                    .map(|i| u16::from_le_bytes([i_data[i], i_data[i + 1]]))
                    .collect();
                String::from_utf16_lossy(&utf16).trim_end_matches('\0').to_string()
            };
            
            // Check if the original path ends with our filename
            let original_name = Path::new(&original_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            
            eprintln!("[RecycleBin] Found $I file: {} -> original: {} (looking for: {})", 
                fname, original_name, file_info.name);
            
            if original_name.to_lowercase() != target_name {
                continue;
            }
            
            // Found a match! The $R file has the same hash as $I
            let r_name = fname.replacen("$I", "$R", 1);
            let r_path = sid_dir.join(&r_name);
            
            if !r_path.exists() {
                eprintln!("[RecycleBin] $R file not found: {}", r_path.display());
                continue;
            }
            
            eprintln!("[RecycleBin] Found matching $R file: {}", r_path.display());
            
            // Copy the $R file to destination
            let dest_path = Path::new(destination);
            if let Some(parent) = dest_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            
            match fs::copy(&r_path, dest_path) {
                Ok(bytes) => {
                    eprintln!("[RecycleBin] Successfully recovered {} bytes from Recycle Bin", bytes);
                    return RecoveryResult {
                        success: true,
                        message: format!("Recovered {} bytes from Recycle Bin (original: {})", bytes, original_path),
                        bytes_recovered: bytes,
                        source_path: original_path,
                        destination_path: destination.to_string(),
                    };
                }
                Err(e) => {
                    eprintln!("[RecycleBin] Failed to copy $R file: {}", e);
                    continue;
                }
            }
        }
    }
    
    eprintln!("[RecycleBin] Scanned {} SID dirs, {} $I files checked, no match found", sid_count, i_files_scanned);
    
    RecoveryResult {
        success: false,
        message: "File not found in Recycle Bin".to_string(),
        bytes_recovered: 0,
        source_path: file_info.path.clone(),
        destination_path: destination.to_string(),
    }
}

/// Recover a file from Volume Shadow Copies (Previous Versions)
/// Uses WMI to enumerate shadow copies and checks each one for the file
fn recover_from_vss(file_info: &FileInfoForRecovery, destination: &str) -> RecoveryResult {
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    
    let source_path = &file_info.path;
    
    // Get shadow copy device paths using vssadmin
    let output = match Command::new("vssadmin")
        .args(&["list", "shadows"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return RecoveryResult {
                success: false,
                message: format!("Cannot enumerate shadow copies: {}", e),
                bytes_recovered: 0,
                source_path: source_path.clone(),
                destination_path: destination.to_string(),
            };
        }
    };
    
    let vss_output = String::from_utf8_lossy(&output.stdout);
    
    // Extract shadow copy volume paths
    let shadow_volumes: Vec<&str> = vss_output
        .lines()
        .filter(|l| l.contains("Shadow Copy Volume:"))
        .filter_map(|l| l.split(':').nth(1).map(|s| s.trim()))
        .collect();
    
    if shadow_volumes.is_empty() {
        return RecoveryResult {
            success: false,
            message: "No Volume Shadow Copies found".to_string(),
            bytes_recovered: 0,
            source_path: source_path.clone(),
            destination_path: destination.to_string(),
        };
    }
    
    eprintln!("[VSS] Found {} shadow copies", shadow_volumes.len());
    
    // Convert source path like "C:\Users\Ankit\Desktop\file.pdf" to relative path
    // "Users\Ankit\Desktop\file.pdf"
    let relative_path = if source_path.len() > 3 && source_path.chars().nth(1) == Some(':') {
        &source_path[3..]  // Skip "C:\"
    } else {
        source_path.as_str()
    };
    
    // Try each shadow copy (newest first — they're listed chronologically)
    for shadow_vol in shadow_volumes.iter().rev() {
        // Construct the full path: \\?\GLOBALROOT\Device\HarddiskVolumeShadowCopyN\relative\path
        let shadow_file = format!("{}:{}", shadow_vol, relative_path);
        // Actually, shadow copies are accessed via symlinks or direct device paths
        // We need to use a temp symlink approach or PowerShell
        
        // Use PowerShell to copy from shadow copy
        let ps_script = format!(
            r#"
            $shadowDevice = '\\?\GLOBALROOT\Device\{}'
            $shadowPath = "$shadowDevice\{}"
            if ([System.IO.File]::Exists($shadowPath)) {{
                [System.IO.File]::Copy($shadowPath, '{}', $true)
                Write-Output "OK"
            }} else {{
                Write-Output "NOT_FOUND"
            }}
            "#,
            shadow_vol.trim().replace("\\\\?\\GLOBALROOT\\Device\\", ""),
            relative_path.replace("\\", "\\"),
            destination.replace("'", "''")
        );
        
        // Simpler approach: create a temp junction and copy
        let device_name = shadow_vol.trim();
        // Extract just the "HarddiskVolumeShadowCopyN" part
        let device_short = if let Some(idx) = device_name.rfind("HarddiskVolumeShadowCopy") {
            &device_name[idx..]
        } else {
            continue;
        };
        
        let junction_path = format!("C:\\__vss_temp_{}", device_short);
        let full_shadow_path = format!("{}\\{}", junction_path, relative_path);
        
        eprintln!("[VSS] Checking shadow copy: {} for {}", device_name, relative_path);
        
        // Create temp symlink
        let mklink = Command::new("cmd")
            .args(&["/c", &format!("mklink /d \"{}\" \"{}\\\"", junction_path, device_name)])
            .output();
        
        if mklink.is_err() {
            continue;
        }
        
        // Check if file exists in shadow copy
        let shadow_file_path = Path::new(&full_shadow_path);
        if shadow_file_path.exists() {
            eprintln!("[VSS] Found file in shadow copy: {}", full_shadow_path);
            
            let dest_path = Path::new(destination);
            if let Some(parent) = dest_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            
            match fs::copy(&shadow_file_path, dest_path) {
                Ok(bytes) => {
                    // Clean up junction
                    let _ = Command::new("cmd")
                        .args(&["/c", &format!("rmdir \"{}\"", junction_path)])
                        .output();
                    
                    return RecoveryResult {
                        success: true,
                        message: format!("Recovered {} bytes from Volume Shadow Copy (Previous Version)", bytes),
                        bytes_recovered: bytes,
                        source_path: source_path.clone(),
                        destination_path: destination.to_string(),
                    };
                }
                Err(e) => {
                    eprintln!("[VSS] Copy from shadow failed: {}", e);
                }
            }
        }
        
        // Clean up junction
        let _ = Command::new("cmd")
            .args(&["/c", &format!("rmdir \"{}\"", junction_path)])
            .output();
    }
    
    RecoveryResult {
        success: false,
        message: "File not found in any Volume Shadow Copy".to_string(),
        bytes_recovered: 0,
        source_path: source_path.clone(),
        destination_path: destination.to_string(),
    }
}

/// Recover a deleted file using cluster-based recovery
fn recover_deleted_file_fs(drive: &str, file_info: &FileInfoForRecovery, destination: &str) -> RecoveryResult {
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    
    let dest_path = Path::new(destination);
    
    // Ensure destination directory exists
    if let Some(parent) = dest_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return RecoveryResult {
                    success: false,
                    message: format!("Failed to create destination directory: {}", e),
                    bytes_recovered: 0,
                    source_path: file_info.path.clone(),
                    destination_path: destination.to_string(),
                };
            }
        }
    }
    
    // For NON-DELETED files, try direct Windows copy (auto-decrypts BitLocker).
    // IMPORTANT: Skip this for deleted files! The path may now point to a different file
    // (or a previously-recovered corrupt file) that has nothing to do with the original.
    let is_deleted = file_info.is_deleted.unwrap_or(true);
    if !is_deleted {
        let source_path = Path::new(&file_info.path);
        if source_path.exists() {
            eprintln!("[FileSystem] Non-deleted file exists on disk, using Windows copy: {}", file_info.path);
            match fs::copy(source_path, dest_path) {
                Ok(bytes) => {
                    return RecoveryResult {
                        success: true,
                        message: format!("Recovered {} bytes via Windows copy (decrypted)", bytes),
                        bytes_recovered: bytes,
                        source_path: file_info.path.clone(),
                        destination_path: destination.to_string(),
                    };
                }
                Err(e) => {
                    eprintln!("[FileSystem] Windows copy failed: {}", e);
                }
            }
        }
    } else {
        eprintln!("[FileSystem] File is deleted, skipping 'file exists' shortcut — using cluster/MFT recovery");
    }
    
    // Parse data runs if available
    eprintln!("[FileSystem] Parsing data_runs: {:?}", file_info.data_runs);
    let data_runs: Vec<DataRun> = if let Some(ref runs_json) = file_info.data_runs {
        match serde_json::from_str(runs_json) {
            Ok(runs) => runs,
            Err(e) => {
                eprintln!("[FileSystem] Failed to parse data_runs: {}", e);
                Vec::new()
            }
        }
    } else {
        eprintln!("[FileSystem] No data_runs provided");
        Vec::new()
    };
    
    // If data_runs is empty, try alternative recovery methods
    if data_runs.is_empty() {
        eprintln!("[FileSystem] No data runs available");
        
        // Try Recycle Bin recovery first (most common for recently deleted files)
        eprintln!("[FileSystem] Attempting Recycle Bin recovery for: {}", file_info.name);
        let recycle_result = recover_from_recycle_bin(drive, file_info, destination);
        if recycle_result.success {
            return recycle_result;
        }
        eprintln!("[FileSystem] Recycle Bin recovery failed: {}", recycle_result.message);
        
        // Try Volume Shadow Copy (Previous Versions) recovery
        eprintln!("[FileSystem] Attempting Volume Shadow Copy recovery");
        let vss_result = recover_from_vss(file_info, destination);
        if vss_result.success && vss_result.bytes_recovered > 0 {
            return vss_result;
        }
        eprintln!("[FileSystem] VSS recovery failed: {}", vss_result.message);
        
        // Then try resident file recovery from MFT
        eprintln!("[FileSystem] Attempting resident file recovery from MFT");
        let resident_result = recover_resident_file(drive, file_info, destination);
        if resident_result.success && resident_result.bytes_recovered > 0 {
            return resident_result;
        }
        
        // Try file carving as last resort (scan volume for file signatures)
        eprintln!("[FileSystem] Attempting file carving (signature-based recovery)");
        let carve_result = carve_file_from_volume(drive, file_info, destination);
        if carve_result.success && carve_result.bytes_recovered > 0 {
            return carve_result;
        }
        eprintln!("[FileSystem] File carving failed: {}", carve_result.message);
        
        // All methods exhausted
        return RecoveryResult {
            success: false,
            message: format!("'{}' cannot be recovered - file data has been overwritten or is no longer accessible on disk", 
                file_info.name),
            bytes_recovered: 0,
            source_path: file_info.path.clone(),
            destination_path: destination.to_string(),
        };
    }
    
    eprintln!("[FileSystem] Found {} data runs", data_runs.len());
    for (i, run) in data_runs.iter().take(3).enumerate() {
        eprintln!("[FileSystem]   Run {}: offset={}, count={}", i, run.cluster_offset, run.cluster_count);
    }
    
    // Try cluster-based recovery using volume access
    let drive_letter = drive.trim_end_matches('\\').trim_end_matches(':');
    
    eprintln!("[FileSystem] Attempting cluster-based recovery for: {}", file_info.name);
    eprintln!("[FileSystem] Drive: {}, Size: {} bytes, Data runs: {:?}", drive_letter, file_info.size, data_runs);
    
    // Create disk reader for the drive
    match crate::filesystem_disk_reader::FileSystemDiskReader::new(drive_letter) {
        Ok(mut reader) => {
            // Read ACTUAL cluster size from the volume's boot sector.
            // Hardcoding 4096 causes corruption on volumes with 8K/16K/64K clusters
            // because data_run byte offsets = cluster_number * cluster_size.
            let cluster_size = match reader.test_access() {
                Ok(_) => {
                    let cs = reader.get_cluster_size();
                    eprintln!("[FileSystem] Actual cluster size from boot sector: {} bytes", cs);
                    cs
                }
                Err(e) => {
                    eprintln!("[FileSystem] Could not read boot sector ({}), using default 4096", e);
                    4096u64
                }
            };
            let mut recovered_data = Vec::with_capacity(file_info.size as usize);
            let mut total_read = 0u64;
            
            for run in &data_runs {
                if run.cluster_offset <= 0 || run.cluster_count <= 0 {
                    continue;
                }
                
                let bytes_to_read = (run.cluster_count as u64) * cluster_size;
                let bytes_needed = file_info.size.saturating_sub(total_read);
                let read_count = bytes_to_read.min(bytes_needed);
                
                if read_count == 0 {
                    break;
                }
                
                let cluster_count = (read_count + cluster_size - 1) / cluster_size;
                
                eprintln!("[FileSystem] Reading {} clusters at offset {}", cluster_count, run.cluster_offset);
                
                match reader.read_clusters(run.cluster_offset as u64, cluster_count, cluster_size) {
                    Ok(data) => {
                        let actual_bytes = data.len().min(bytes_needed as usize);
                        recovered_data.extend_from_slice(&data[..actual_bytes]);
                        total_read += actual_bytes as u64;
                    }
                    Err(e) => {
                        eprintln!("[FileSystem] Failed to read clusters: {}", e);
                        // Continue with partial recovery
                        break;
                    }
                }
                
                if total_read >= file_info.size {
                    break;
                }
            }
            
            if recovered_data.is_empty() {
                // Cluster read failed — try Recycle Bin as fallback
                eprintln!("[FileSystem] Cluster data empty, trying Recycle Bin fallback");
                let recycle_result = recover_from_recycle_bin(drive, file_info, destination);
                if recycle_result.success {
                    return recycle_result;
                }
                return RecoveryResult {
                    success: false,
                    message: "Failed to read cluster data and file not found in Recycle Bin".to_string(),
                    bytes_recovered: 0,
                    source_path: file_info.path.clone(),
                    destination_path: destination.to_string(),
                };
            }
            
            // Truncate to actual file size
            if recovered_data.len() > file_info.size as usize {
                recovered_data.truncate(file_info.size as usize);
            }
            
            // Write to destination
            match fs::File::create(dest_path) {
                Ok(mut file) => {
                    match file.write_all(&recovered_data) {
                        Ok(_) => {
                            RecoveryResult {
                                success: true,
                                message: format!("Recovered {} bytes via cluster recovery", recovered_data.len()),
                                bytes_recovered: recovered_data.len() as u64,
                                source_path: file_info.path.clone(),
                                destination_path: destination.to_string(),
                            }
                        }
                        Err(e) => {
                            RecoveryResult {
                                success: false,
                                message: format!("Failed to write file: {}", e),
                                bytes_recovered: 0,
                                source_path: file_info.path.clone(),
                                destination_path: destination.to_string(),
                            }
                        }
                    }
                }
                Err(e) => {
                    RecoveryResult {
                        success: false,
                        message: format!("Failed to create output file: {}", e),
                        bytes_recovered: 0,
                        source_path: file_info.path.clone(),
                        destination_path: destination.to_string(),
                    }
                }
            }
        }
        Err(e) => {
            RecoveryResult {
                success: false,
                message: format!("Failed to open drive for recovery: {}", e),
                bytes_recovered: 0,
                source_path: file_info.path.clone(),
                destination_path: destination.to_string(),
            }
        }
    }
}

/// Recover a file using filesystem mode
/// For non-deleted files: Uses Windows copy (auto-decrypts BitLocker)
/// For deleted files: Uses cluster-based recovery
fn recover_file_fs(source: &str, destination: &str) -> RecoveryResult {
    use std::fs;
    use std::path::Path;
    
    let source_path = Path::new(source);
    let dest_path = Path::new(destination);
    
    // Ensure destination directory exists
    if let Some(parent) = dest_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return RecoveryResult {
                    success: false,
                    message: format!("Failed to create destination directory: {}", e),
                    bytes_recovered: 0,
                    source_path: source.to_string(),
                    destination_path: destination.to_string(),
                };
            }
        }
    }
    
    // Check if source file exists on disk
    if source_path.exists() {
        // Non-deleted file: Use Windows copy (BitLocker auto-decryption)
        eprintln!("[FileSystem] Copying file via Windows (auto-decrypt): {}", source);
        
        match fs::copy(source_path, dest_path) {
            Ok(bytes) => RecoveryResult {
                success: true,
                message: format!("Successfully recovered {} bytes (decrypted)", bytes),
                bytes_recovered: bytes,
                source_path: source.to_string(),
                destination_path: destination.to_string(),
            },
            Err(e) => RecoveryResult {
                success: false,
                message: format!("Failed to copy file: {}", e),
                bytes_recovered: 0,
                source_path: source.to_string(),
                destination_path: destination.to_string(),
            }
        }
    } else {
        // Deleted file or MFT path - need cluster-based recovery
        RecoveryResult {
            success: false,
            message: "File not found on disk. Deleted file recovery requires cluster-level access.".to_string(),
            bytes_recovered: 0,
            source_path: source.to_string(),
            destination_path: destination.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct RecoveryResult {
    success: bool,
    message: String,
    bytes_recovered: u64,
    source_path: String,
    destination_path: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }
    
    let command = &args[1];
    
    match command.as_str() {
        "drives" => {
            let drives = get_drives();
            let json = serde_json::to_string(&drives).unwrap();
            println!("{}", json);
        }
        
        "check-admin" => {
            let status = AdminStatus {
                is_admin: is_admin(),
                message: if is_admin() {
                    "Running with administrator privileges".to_string()
                } else {
                    "Not running as administrator. Elevated privileges required for recovery.".to_string()
                },
            };
            let json = serde_json::to_string(&status).unwrap();
            println!("{}", json);
        }
        
        "bitlocker-status" => {
            if args.len() < 3 {
                eprintln!("Usage: data_recovery_filesystem bitlocker-status <drive>");
                std::process::exit(1);
            }
            let drive = &args[2];
            let status = get_bitlocker_status(drive);
            let json = serde_json::to_string(&status).unwrap();
            println!("{}", json);
        }
        
        "bitlocker-unlock-password" => {
            if args.len() < 4 {
                eprintln!("Usage: data_recovery_filesystem bitlocker-unlock-password <drive> <password>");
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
                eprintln!("Usage: data_recovery_filesystem bitlocker-unlock-key <drive> <recovery_key>");
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
                eprintln!("Usage: data_recovery_filesystem bitlocker-lock <drive>");
                std::process::exit(1);
            }
            let drive = &args[2];
            let result = lock_drive(drive);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
        }
        
        // FileSystem Recovery Commands
        "scan" | "deep-scan" => {
            if args.len() < 3 {
                eprintln!("Usage: data_recovery_filesystem scan <drive>");
                std::process::exit(1);
            }
            let drive = &args[2];
            let mode = args.get(3).map(|s| s.as_str()).unwrap_or("quick");
            
            eprintln!("[FileSystem Backend] Scanning encrypted drive using file system APIs...");
            let result = perform_scan_filesystem(drive, mode);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
            
            if !result.success {
                std::process::exit(1);
            }
        }
        
        "recover" => {
            if args.len() < 4 {
                eprintln!("Usage: data_recovery_filesystem recover <source_path> <destination>");
                std::process::exit(1);
            }
            let source = &args[2];
            let destination = &args[3];
            
            eprintln!("[FileSystem Backend] Recovering file: {} -> {}", source, destination);
            let result = recover_file_fs(source, destination);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
            
            if !result.success {
                std::process::exit(1);
            }
        }
        
        "recover-deleted" => {
            if args.len() < 5 {
                eprintln!("Usage: data_recovery_filesystem recover-deleted <drive> <file_info_json> <destination>");
                eprintln!("       file_info_json can be @filepath to read from a file");
                std::process::exit(1);
            }
            let drive = &args[2];
            let file_info_json_arg = &args[3];
            let destination = &args[4];
            
            // Support @filepath to read JSON from a file (useful for testing)
            let file_info_json = if file_info_json_arg.starts_with('@') {
                match std::fs::read_to_string(&file_info_json_arg[1..]) {
                    Ok(contents) => contents,
                    Err(e) => {
                        let result = RecoveryResult {
                            success: false,
                            message: format!("Failed to read file info from {}: {}", &file_info_json_arg[1..], e),
                            bytes_recovered: 0,
                            source_path: "unknown".to_string(),
                            destination_path: destination.to_string(),
                        };
                        println!("{}", serde_json::to_string(&result).unwrap());
                        std::process::exit(1);
                    }
                }
            } else {
                file_info_json_arg.to_string()
            };
            
            // Parse file info JSON
            let file_info: FileInfoForRecovery = match serde_json::from_str(&file_info_json) {
                Ok(info) => info,
                Err(e) => {
                    let result = RecoveryResult {
                        success: false,
                        message: format!("Failed to parse file info: {}", e),
                        bytes_recovered: 0,
                        source_path: "unknown".to_string(),
                        destination_path: destination.to_string(),
                    };
                    println!("{}", serde_json::to_string(&result).unwrap());
                    std::process::exit(1);
                }
            };
            
            eprintln!("[FileSystem Backend] Recovering deleted file: {} -> {}", file_info.name, destination);
            let result = recover_deleted_file_fs(drive, &file_info, destination);
            let json = serde_json::to_string(&result).unwrap();
            println!("{}", json);
            
            if !result.success {
                std::process::exit(1);
            }
        }
        
        "help" | "--help" | "-h" => {
            print_usage();
        }
        
        "version" | "--version" | "-v" => {
            println!("RecoverPro FileSystem Backend v2.0.0");
            println!("Optimized for BitLocker Encrypted Drives");
            println!("Uses Windows decryption layer via $MFT file system access");
        }
        
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("
RecoverPro FileSystem Backend v2.0.0
=====================================
Optimized for BitLocker encrypted drives

COMMANDS:
  drives                          List all available drives
  check-admin                     Check if running as administrator
  
BITLOCKER:
  bitlocker-status <drive>        Check BitLocker status
  bitlocker-unlock-password <drive> <password>
                                  Unlock with password
  bitlocker-unlock-key <drive> <key>
                                  Unlock with recovery key
  bitlocker-lock <drive>          Lock a BitLocker drive

RECOVERY (FileSystem Mode):
  scan <drive>                    Scan encrypted drive for files
  deep-scan <drive>               Same as scan (FileSystem mode)
  recover <source> <destination>  Recover a file

OTHER:
  help, --help, -h                Show this help
  version, --version, -v          Show version

HOW IT WORKS:
  This backend reads the $MFT file through Windows file system APIs.
  Windows automatically decrypts the data for BitLocker drives.
  No raw disk access - works on encrypted drives!

USE CASES:
  - BitLocker encrypted drives (unlocked)
  - Drives where raw disk access fails
  - When you need decrypted data access
");
}
