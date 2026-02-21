//! File System Recovery Engine
//! Optimized for encrypted drives (BitLocker) using file system APIs
//! Works through Windows decryption layer instead of raw disk access

use crate::bitlocker::{get_bitlocker_status, is_admin, BitLockerStatus};
use crate::filesystem_disk_reader::{FileSystemDiskReader, UsnDeletedFile};
use crate::ntfs_parser::{parse_mft_record, MftEntry};

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Result of a filesystem recovery scan
#[derive(Serialize, Deserialize, Debug)]
pub struct FileSystemScanResult {
    pub success: bool,
    pub message: String,
    pub scan_mode: String,
    pub drive: String,
    pub bitlocker_status: Option<BitLockerStatus>,
    pub mft_entries: Vec<RecoverableFileFS>,
    pub total_files: usize,
    pub total_recoverable_size: u64,
    pub scan_duration_ms: u64,
    pub mft_records_scanned: u64,
    pub requires_admin: bool,
}

/// A file that can be recovered via file system mode
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecoverableFileFS {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub extension: String,
    pub category: String,
    pub file_type: String,
    pub modified: String,
    pub created: String,
    pub is_deleted: bool,
    pub recovery_chance: u8,
    pub source: String,
    pub cluster_offset: Option<i64>,
    pub data_runs: Option<String>,
}

/// Recovery result for a single file
#[derive(Serialize, Deserialize, Debug)]
pub struct FileRecoveryResultFS {
    pub success: bool,
    pub source_path: String,
    pub destination_path: String,
    pub bytes_recovered: u64,
    pub message: String,
}

/// File system-based recovery engine for encrypted drives
pub struct FileSystemRecoveryEngine {
    drive_letter: String,
    disk_reader: Option<FileSystemDiskReader>,
    cancelled: Arc<AtomicBool>,
    files_found: Arc<AtomicU64>,
    cluster_size: u64,
}

impl FileSystemRecoveryEngine {
    /// Create a new file system recovery engine
    pub fn new(drive_letter: &str) -> Self {
        let letter = drive_letter
            .trim_end_matches('\\')
            .trim_end_matches(':')
            .to_uppercase();
        
        FileSystemRecoveryEngine {
            drive_letter: letter,
            disk_reader: None,
            cancelled: Arc::new(AtomicBool::new(false)),
            files_found: Arc::new(AtomicU64::new(0)),
            cluster_size: 4096, // Default NTFS cluster size
        }
    }
    
    /// Check if admin privileges are available
    pub fn check_admin(&self) -> bool {
        is_admin()
    }
    
    /// Check BitLocker status
    pub fn check_bitlocker(&self) -> BitLockerStatus {
        get_bitlocker_status(&self.drive_letter)
    }
    
    /// Initialize file system access
    pub fn initialize(&mut self) -> Result<(), String> {
        // Check admin privileges
        if !is_admin() {
            return Err("Administrator privileges required. Please run as Administrator.".to_string());
        }
        
        // Check BitLocker status - must be UNLOCKED (not necessarily decrypted)
        let bl_status = self.check_bitlocker();
        if bl_status.is_locked {
            return Err(format!(
                "Drive {} is BitLocker encrypted and locked. Please unlock it first.",
                self.drive_letter
            ));
        }
        
        // Create file system disk reader
        let mut reader = FileSystemDiskReader::new(&self.drive_letter)?;
        
        // Test access (also reads boot sector → sets cluster_size)
        eprintln!("DEBUG [FS]: Testing file system access to drive {}...", self.drive_letter);
        reader.test_access()?;
        eprintln!("DEBUG [FS]: File system access confirmed (decryption layer active)");
        
        // Use actual cluster size from boot sector instead of hardcoded default
        let actual_cluster_size = reader.get_cluster_size();
        if actual_cluster_size != self.cluster_size {
            eprintln!("DEBUG [FS]: Cluster size updated: {} -> {} bytes", self.cluster_size, actual_cluster_size);
            self.cluster_size = actual_cluster_size;
        }
        
        self.disk_reader = Some(reader);
        Ok(())
    }
    
    /// Perform MFT scan using file system APIs
    /// 
    /// Parameters:
    /// - max_records: Maximum number of MFT records to scan (10K for quick, 50K for deep)
    /// - hours_limit: Optional flag to indicate quick scan mode (not used for filtering, just logging)
    pub fn scan_mft(&mut self, max_records: Option<usize>, hours_limit: Option<u64>) -> Result<FileSystemScanResult, String> {
        let start_time = std::time::Instant::now();
        
        eprintln!("DEBUG [FS]: Starting file system scan...");
        if let Err(e) = self.initialize() {
            eprintln!("DEBUG [FS]: Initialize failed: {}", e);
            return Err(e);
        }
        
        let bl_status = self.check_bitlocker();
        let mut reader = self.disk_reader.as_mut().unwrap();
        
        let mut total_size = 0u64;
        let mut scanned = 0u64;
        let mut records_read = 0u64;
        let mut records_with_signature = 0u64;
        let mut deleted_count_scan = 0u64;
        
        // Calculate scan limit based on mode
        // IMPORTANT: Limit to prevent massive JSON responses
        let max_limit = max_records.unwrap_or(50000); // Default to 50K max
        let mft_total = reader.get_mft_total_records().unwrap_or(0);
        
        let limit = if mft_total > 0 {
            // Cap at max_limit to prevent memory issues
            std::cmp::min(mft_total, max_limit as u64) as usize
        } else {
            max_limit
        };
        
        // Scan MFT records
        let scan_type = if hours_limit.is_some() { "quick" } else { "deep" };
        eprintln!("DEBUG [FS]: Starting {} scan through file system API (decrypted)...", scan_type);
        eprintln!("DEBUG [FS]: MFT has {} total records, scanning up to {}", mft_total, limit);
        
        let mut record_num = 0u64;
        // Increased from 100 to 5000 - MFT can have large gaps of zeroed records
        let mut consecutive_failures = 0;
        let max_consecutive_failures = 5000;
        
        // Collect all entries first
        let mut parsed_entries: Vec<MftEntry> = Vec::new();
        // Build directory map: record_number -> (parent_record, name)
        let mut dir_map: std::collections::HashMap<u64, (u64, String)> = std::collections::HashMap::new();
        
        while record_num < limit as u64 && consecutive_failures < max_consecutive_failures {
            if self.cancelled.load(Ordering::Relaxed) {
                break;
            }
            
            match reader.read_mft_record(record_num) {
                Ok(buffer) => {
                    consecutive_failures = 0;
                    scanned += 1;
                    records_read += 1;
                    
                    // Check if this has FILE signature
                    if buffer.len() >= 4 && &buffer[0..4] == b"FILE" {
                        records_with_signature += 1;
                    }
                    
                    // Parse the decrypted MFT record
                    if let Some(entry) = parse_mft_record(&buffer, record_num) {
                        // Log deleted files for debugging
                        if entry.is_deleted && !entry.is_directory {
                            deleted_count_scan += 1;
                            if deleted_count_scan <= 10 {  // Only log first 10
                                eprintln!("DELETED FILE FOUND: {} (record {}, size {})", entry.file_name, entry.record_number, entry.file_size);
                            }
                        }
                        if entry.is_deleted && entry.is_directory {
                            eprintln!("DELETED FOLDER FOUND: {} (record {})", entry.file_name, entry.record_number);
                        }
                        
                        // Add ALL directories to map (even deleted ones) for path resolution
                        if entry.is_directory {
                            dir_map.insert(entry.record_number, (entry.parent_record, entry.file_name.clone()));
                        }
                        parsed_entries.push(entry);
                    }
                }
                Err(_) => {
                    consecutive_failures += 1;
                }
            }
            
            record_num += 1;
            
            // Progress reporting - log when we hit key milestones
            if record_num % 50000 == 0 {
                eprintln!("DEBUG [FS]: {} records read | {} with FILE sig | {} parsed | {} deleted", 
                    records_read, records_with_signature, parsed_entries.len(), deleted_count_scan);
            }
        }
        
        let deleted_count = parsed_entries.iter().filter(|e| e.is_deleted && !e.is_directory).count();
        let deleted_dirs_count = parsed_entries.iter().filter(|e| e.is_deleted && e.is_directory).count();
        eprintln!("\n=== MFT SCAN SUMMARY ===");
        eprintln!("Total MFT records scanned: {}", scanned);
        eprintln!("Total entries parsed: {}", parsed_entries.len());
        eprintln!("Active files/folders: {}", parsed_entries.len() - deleted_count - deleted_dirs_count);
        eprintln!("DELETED FILES FOUND: {}", deleted_count);
        eprintln!("DELETED FOLDERS FOUND: {}", deleted_dirs_count);
        eprintln!("========================\n");
        
        eprintln!("DEBUG [FS]: Building file paths from {} directory entries...", dir_map.len());
        
        // Note: Quick scan speed comes from scanning fewer MFT records (10K vs 50K)
        // We don't filter by time because MFT modified_time != deletion time
        // A file modified 1 year ago but deleted today still shows old modified_time
        let scan_type_note = if hours_limit.is_some() {
            "quick scan (up to 10K records for speed)"
        } else {
            "deep scan (up to 50K records)"
        };
        eprintln!("DEBUG [FS]: Processing {} parsed entries ({})...", parsed_entries.len(), scan_type_note);
        
        // Now convert parsed entries to recoverable files with proper paths
        let mut mft_entries = Vec::new();
        
        for entry in &parsed_entries {
            if let Some(file) = mft_entry_to_recoverable_with_path(&self.drive_letter, entry, &dir_map) {
                total_size += file.size;
                mft_entries.push(file);
                self.files_found.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        eprintln!("DEBUG [FS]: MFT converted to {} recoverable files", mft_entries.len());
        
        // ===================================================================
        // USN JOURNAL SCAN: Find recently deleted files (critical for BitLocker)
        // The MFT scan on BitLocker drives often can't see deleted entries
        // because Windows reuses MFT records quickly. The USN Change Journal  
        // keeps a log of ALL file deletions and works through the volume API.
        // ===================================================================
        eprintln!("DEBUG [USN]: Starting USN Change Journal scan for deleted files...");
        
        match reader.scan_usn_journal() {
            Ok(usn_deleted) => {
                eprintln!("DEBUG [USN]: Found {} deleted files in USN journal", usn_deleted.len());
                
                // Deduplicate: only add files not already in results
                let existing_mft_records: std::collections::HashSet<u64> = mft_entries.iter()
                    .map(|f| {
                        // Extract MFT record from ID like "fs_mft_12345"
                        f.id.strip_prefix("fs_mft_").and_then(|s| s.parse().ok()).unwrap_or(0)
                    })
                    .collect();
                
                let mut usn_added = 0;
                let mut seen_usn_records: std::collections::HashSet<u64> = std::collections::HashSet::new();
                for usn_file in &usn_deleted {
                    // Skip if this MFT record already has an entry from the MFT scan
                    if existing_mft_records.contains(&usn_file.mft_record) {
                        continue;
                    }
                    // Skip if we already added this MFT record from an earlier USN event
                    if !seen_usn_records.insert(usn_file.mft_record) {
                        continue;
                    }
                    
                    // Skip system/temp files
                    let name_lower = usn_file.file_name.to_lowercase();
                    if name_lower.starts_with("$") || name_lower.starts_with("~") {
                        continue;
                    }
                    
                    // Build path from parent record using dir_map
                    let mut full_path = build_full_path(
                        &self.drive_letter, 
                        usn_file.parent_mft_record, 
                        &usn_file.file_name, 
                        &dir_map
                    );
                    
                    // If path resolution failed (just drive:\filename), try reading parent MFT records directly
                    let just_root = format!("{}:\\{}", self.drive_letter, usn_file.file_name);
                    if full_path == just_root && usn_file.parent_mft_record != 5 {
                        let resolved = resolve_path_from_mft(
                            &mut reader,
                            &self.drive_letter,
                            usn_file.parent_mft_record,
                            &usn_file.file_name,
                        );
                        if resolved != full_path {
                            full_path = resolved;
                        }
                    }
                    
                    // Get extension
                    let extension = if let Some(pos) = usn_file.file_name.rfind('.') {
                        usn_file.file_name[pos + 1..].to_lowercase()
                    } else {
                        String::new()
                    };
                    
                    // Filter out Windows shortcuts and temporary files
                    match extension.as_str() {
                        "lnk" | "url" | "ini" | "tmp" | "temp" | "log" | "bak" | "thumbs" => continue,
                        _ => {}
                    }
                    
                    let category = categorize_file(&extension);
                    let file_type = extension.clone();
                    
                    // Try to read MFT record to see if data runs still exist
                    let (file_size, recovery_chance, data_runs_json, first_cluster) = 
                        match reader.read_mft_record(usn_file.mft_record) {
                            Ok(buffer) => {
                                // Debug: check first 4 bytes (should be "FILE")
                                let sig = String::from_utf8_lossy(&buffer[0..4]);
                                eprintln!("DEBUG [USN-MFT]: Record {} for '{}': signature={:?}, first_bytes={:02x} {:02x} {:02x} {:02x}",
                                    usn_file.mft_record, usn_file.file_name, sig,
                                    buffer[0], buffer[1], buffer[2], buffer[3]);
                                
                                if let Some(mft_entry) = parse_mft_record(&buffer, usn_file.mft_record) {
                                    eprintln!("DEBUG [USN-MFT]: Parsed OK - name='{}', deleted={}, size={}, data_runs={}",
                                        mft_entry.file_name, mft_entry.is_deleted, mft_entry.file_size, mft_entry.data_runs.len());
                                    
                                    // MFT record exists but might be reused
                                    if mft_entry.is_deleted || mft_entry.file_name == usn_file.file_name {
                                        // Record still has our deleted file's data
                                        let chance = if mft_entry.data_runs.is_empty() { 15 } else { 65 };
                                        let runs_json = serde_json::to_string(&mft_entry.data_runs)
                                            .unwrap_or_else(|_| "[]".to_string());
                                        let first = mft_entry.data_runs.first().map(|r| r.cluster_offset);
                                        (mft_entry.file_size, chance, runs_json, first)
                                    } else {
                                        // MFT record reused for different file
                                        eprintln!("DEBUG [USN-MFT]: Record {} REUSED - now contains '{}', not '{}'",
                                            usn_file.mft_record, mft_entry.file_name, usn_file.file_name);
                                        (0, 5, "[]".to_string(), None)
                                    }
                                } else {
                                    eprintln!("DEBUG [USN-MFT]: Record {} PARSE FAILED (no FILE signature or corrupt)", 
                                        usn_file.mft_record);
                                    (0, 10, "[]".to_string(), None)
                                }
                            }
                            Err(_) => (0, 5, "[]".to_string(), None),
                        };
                    
                    // Format deletion timestamp
                    let deletion_time = format_timestamp(usn_file.timestamp);
                    
                    // Skip ONLY Windows system files with 0 bytes AND no interesting extension
                    // Do NOT skip user files even if file_size == 0 (MFT record may have been reused
                    // but the file was real and recently deleted - show it so user knows it was there)
                    let is_likely_system = file_size == 0 && (
                        name_lower.contains(".tmp") ||
                        name_lower.contains(".log") ||
                        name_lower.contains(".temp")
                    );
                    if is_likely_system {
                        continue;
                    }
                    
                    // If data_runs are gone but file existed, still show it with a low recovery chance
                    // The user deserves to know it was deleted recently, even if recovery is hard
                    let final_recovery_chance = if file_size == 0 {
                        // MFT record reused - disk sectors may still have old data but we can't confirm
                        3u8
                    } else if data_runs_json == "[]" {
                        // File exists in USN but cluster chain is lost
                        8u8
                    } else {
                        recovery_chance
                    };
                    
                    // Use a placeholder size when MFT was reused (we know file existed but not exact size)
                    // Mark clearly as "MFT record reused" in path
                    let final_size = file_size; // 0 is valid — means we can't confirm size
                    
                    let recoverable = RecoverableFileFS {
                        id: format!("usn_mft_{}", usn_file.mft_record),
                        name: usn_file.file_name.clone(),
                        path: full_path,
                        size: final_size,
                        extension: extension.clone(),
                        category,
                        file_type,
                        modified: deletion_time.clone(),
                        created: deletion_time,
                        is_deleted: true,
                        recovery_chance: final_recovery_chance,
                        source: "USN".to_string(),
                        cluster_offset: first_cluster,
                        data_runs: if data_runs_json != "[]" { Some(data_runs_json) } else { None },
                    };
                    
                    total_size += final_size;
                    mft_entries.push(recoverable);
                    usn_added += 1;
                }
                
                eprintln!("DEBUG [USN]: Added {} unique deleted files from USN journal", usn_added);
            }
            Err(e) => {
                eprintln!("DEBUG [USN]: USN journal scan failed: {} (non-critical, continuing)", e);
            }
        }
        
        // Sort by relevance for recently deleted files
        if hours_limit.is_some() {
            // Quick scan: Prioritize recent files
            // 1. User folders (Desktop, Documents, Downloads, Pictures) come first
            // 2. Within same priority, higher MFT record # = more recently created/deleted
            mft_entries.sort_by(|a, b| {
                let a_priority = get_path_priority(&a.path);
                let b_priority = get_path_priority(&b.path);
                
                if a_priority != b_priority {
                    a_priority.cmp(&b_priority) // Lower priority value = higher priority
                } else {
                    // Same priority - sort by MFT record number (extract from ID)
                    let a_record = extract_mft_record(&a.id);
                    let b_record = extract_mft_record(&b.id);
                    b_record.cmp(&a_record) // Higher record number first
                }
            });
        } else {
            // Deep scan: Sort by modified date
            mft_entries.sort_by(|a, b| b.modified.cmp(&a.modified));
        }
        
        eprintln!("DEBUG [FS]: Filtered to {} recoverable files (removed 0-byte, temp files, unrecoverable)", mft_entries.len());

        let duration = start_time.elapsed();
        
        let scan_summary = if hours_limit.is_some() {
            format!("Quick scan (scanned {} records)", scanned)
        } else {
            format!("Deep scan (scanned {} records)", scanned)
        };
        
        eprintln!("DEBUG [FS]: {} complete - {} files found in {:.2}s", 
            scan_summary, mft_entries.len(), duration.as_secs_f32());
        
        // Check if we hit the limit (more MFT records exist than we scanned)
        let limit_note = if mft_total > 0 && scanned >= limit as u64 && mft_total > scanned {
            format!("\nNote: Results limited to {} records out of {} total MFT entries. Use filters to refine your search.", 
                scanned, mft_total)
        } else {
            String::new()
        };
        
        Ok(FileSystemScanResult {
            success: true,
            message: format!("Found {} recoverable files (encrypted drive mode){}", 
                mft_entries.len(), limit_note),
            scan_mode: "FileSystem".to_string(),
            drive: self.drive_letter.clone(),
            bitlocker_status: Some(bl_status),
            total_files: mft_entries.len(),
            total_recoverable_size: total_size,
            scan_duration_ms: duration.as_millis() as u64,
            mft_records_scanned: scanned,
            mft_entries,
            requires_admin: true,
        })
    }
    
    /// Recover a file using file system access
    ///
    /// Recovery strategy (tried in order):
    /// 1. Direct Windows copy — works for non-deleted / existing files (auto-decrypts BitLocker)
    /// 2. Cluster-based recovery via data runs — reads raw clusters from the volume
    /// 3. MFT resident data extraction — for small files stored inline in the MFT record
    pub fn recover_file(&mut self, file: &RecoverableFileFS, output_path: &str) -> Result<FileRecoveryResultFS, String> {
        // Ensure destination directory exists
        if let Some(parent) = std::path::Path::new(output_path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create destination directory: {}", e))?;
            }
        }

        // --- Method 1: Direct file copy (non-deleted files that still exist on disk) ---
        if !file.is_deleted {
            let source_path = std::path::Path::new(&file.path);
            if source_path.exists() {
                match std::fs::copy(source_path, output_path) {
                    Ok(bytes) => {
                        return Ok(FileRecoveryResultFS {
                            success: true,
                            source_path: file.path.clone(),
                            destination_path: output_path.to_string(),
                            bytes_recovered: bytes,
                            message: format!("Recovered {} bytes via direct copy (auto-decrypted)", bytes),
                        });
                    }
                    Err(e) => {
                        eprintln!("[Recovery] Direct copy failed for '{}': {}, falling through to cluster recovery", file.name, e);
                    }
                }
            }
        }

        let reader = self.disk_reader.as_mut()
            .ok_or("Recovery engine not initialized. Call initialize() first.")?;

        // --- Method 2: Cluster-based recovery using data runs ---
        let data_runs: Vec<crate::ntfs_parser::DataRun> = if let Some(ref runs_json) = file.data_runs {
            serde_json::from_str(runs_json).unwrap_or_default()
        } else {
            Vec::new()
        };

        if !data_runs.is_empty() && file.size > 0 {
            eprintln!("[Recovery] Attempting cluster recovery for '{}' ({} bytes, {} data runs)",
                file.name, file.size, data_runs.len());

            let mut recovered_data: Vec<u8> = Vec::with_capacity(file.size as usize);
            let mut bytes_remaining = file.size;
            let mut failed_runs = 0u32;
            let mut successful_runs = 0u32;

            for (i, run) in data_runs.iter().enumerate() {
                if bytes_remaining == 0 {
                    break;
                }

                if run.cluster_offset <= 0 {
                    // Sparse run — fill with zeros to preserve file structure
                    let sparse_size = (run.cluster_count * self.cluster_size).min(bytes_remaining);
                    recovered_data.extend(vec![0u8; sparse_size as usize]);
                    bytes_remaining = bytes_remaining.saturating_sub(sparse_size);
                    continue;
                }

                // Only read as many clusters as we still need
                let clusters_needed = (bytes_remaining + self.cluster_size - 1) / self.cluster_size;
                let cluster_count = clusters_needed.min(run.cluster_count);

                match reader.read_clusters(run.cluster_offset as u64, cluster_count, self.cluster_size) {
                    Ok(data) => {
                        let to_take = bytes_remaining.min(data.len() as u64) as usize;
                        recovered_data.extend_from_slice(&data[..to_take]);
                        bytes_remaining = bytes_remaining.saturating_sub(to_take as u64);
                        successful_runs += 1;
                    }
                    Err(e) => {
                        eprintln!("[Recovery] Data run {} failed (cluster offset {}): {}",
                            i, run.cluster_offset, e);
                        // Fill failed section with zeros for partial recovery
                        let failed_size = (run.cluster_count * self.cluster_size).min(bytes_remaining);
                        recovered_data.extend(vec![0u8; failed_size as usize]);
                        bytes_remaining = bytes_remaining.saturating_sub(failed_size);
                        failed_runs += 1;
                    }
                }
            }

            if !recovered_data.is_empty() {
                // Truncate to exact file size (clusters may overread)
                if recovered_data.len() > file.size as usize {
                    recovered_data.truncate(file.size as usize);
                }

                reader.save_file(&recovered_data, output_path)?;

                let message = if failed_runs > 0 {
                    format!(
                        "Partially recovered {} of {} bytes ({:.1}%). {} run(s) succeeded, {} failed.",
                        recovered_data.len(),
                        file.size,
                        (recovered_data.len() as f64 / file.size.max(1) as f64) * 100.0,
                        successful_runs,
                        failed_runs
                    )
                } else {
                    format!("Successfully recovered {} bytes", recovered_data.len())
                };

                return Ok(FileRecoveryResultFS {
                    success: true,
                    source_path: file.path.clone(),
                    destination_path: output_path.to_string(),
                    bytes_recovered: recovered_data.len() as u64,
                    message,
                });
            }
        }

        // --- Method 3: MFT resident data recovery (small files stored inline) ---
        // NTFS stores files smaller than ~700 bytes directly inside the MFT record's
        // $DATA attribute, so no cluster allocation exists for these files.
        if file.size > 0 && file.size <= 700 {
            if let Some(mft_record_num) = extract_mft_record_from_id(&file.id) {
                eprintln!("[Recovery] Attempting MFT resident data recovery for '{}' (record {})",
                    file.name, mft_record_num);

                match reader.read_mft_record(mft_record_num) {
                    Ok(buffer) => {
                        if let Some(resident_data) = extract_resident_data(&buffer, file.size) {
                            reader.save_file(&resident_data, output_path)?;

                            return Ok(FileRecoveryResultFS {
                                success: true,
                                source_path: file.path.clone(),
                                destination_path: output_path.to_string(),
                                bytes_recovered: resident_data.len() as u64,
                                message: format!("Recovered {} bytes from MFT resident data", resident_data.len()),
                            });
                        } else {
                            eprintln!("[Recovery] No resident $DATA attribute found in MFT record {}", mft_record_num);
                        }
                    }
                    Err(e) => {
                        eprintln!("[Recovery] Failed to read MFT record {}: {}", mft_record_num, e);
                    }
                }
            }
        }

        // --- All recovery methods exhausted ---
        let reason = if data_runs.is_empty() {
            "No cluster data available — the MFT record may have been reused by Windows."
        } else {
            "All cluster reads failed — the file data may have been overwritten."
        };

        Ok(FileRecoveryResultFS {
            success: false,
            source_path: file.path.clone(),
            destination_path: output_path.to_string(),
            bytes_recovered: 0,
            message: format!("Could not recover '{}'. {}", file.name, reason),
        })
    }
    
    /// Cancel the current scan
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    
    /// Get number of files found
    pub fn files_found(&self) -> u64 {
        self.files_found.load(Ordering::Relaxed)
    }
}

/// Extract MFT record number from file ID (format: "fs_mft_12345")
fn extract_mft_record(id: &str) -> u64 {
    extract_mft_record_from_id(id).unwrap_or(0)
}

/// Extract MFT record number from any file ID format
/// Supports: "fs_mft_12345", "usn_mft_12345"
fn extract_mft_record_from_id(id: &str) -> Option<u64> {
    id.strip_prefix("fs_mft_")
        .or_else(|| id.strip_prefix("usn_mft_"))
        .and_then(|s| s.parse::<u64>().ok())
}

/// Extract resident file data from an MFT record's $DATA attribute.
///
/// NTFS stores small files (typically ≤700 bytes) directly inside the MFT record
/// rather than allocating separate clusters. This function parses the MFT record
/// to find the unnamed resident $DATA attribute and returns its content.
fn extract_resident_data(mft_record: &[u8], expected_size: u64) -> Option<Vec<u8>> {
    // Minimum valid MFT record size
    if mft_record.len() < 56 {
        return None;
    }

    // Verify FILE signature
    if &mft_record[0..4] != b"FILE" {
        return None;
    }

    // First attribute offset at 0x14
    let first_attr_offset = u16::from_le_bytes([mft_record[0x14], mft_record[0x15]]) as usize;
    let mut offset = first_attr_offset;

    while offset + 24 < mft_record.len() {
        let attr_type = u32::from_le_bytes([
            mft_record[offset],
            mft_record[offset + 1],
            mft_record[offset + 2],
            mft_record[offset + 3],
        ]);

        // End-of-attributes marker
        if attr_type == 0xFFFFFFFF || attr_type == 0 {
            break;
        }

        let attr_length = u32::from_le_bytes([
            mft_record[offset + 4],
            mft_record[offset + 5],
            mft_record[offset + 6],
            mft_record[offset + 7],
        ]) as usize;

        if attr_length == 0 || offset + attr_length > mft_record.len() {
            break;
        }

        // $DATA attribute = 0x80
        if attr_type == 0x80 {
            let non_resident = mft_record[offset + 8];
            let name_length = mft_record[offset + 9];

            // Only process the unnamed (default) $DATA stream that is resident
            if non_resident == 0 && name_length == 0 {
                // Resident attribute: content size at offset+16, content offset at offset+20
                let content_size = u32::from_le_bytes([
                    mft_record[offset + 16],
                    mft_record[offset + 17],
                    mft_record[offset + 18],
                    mft_record[offset + 19],
                ]) as usize;

                let content_offset = u16::from_le_bytes([
                    mft_record[offset + 20],
                    mft_record[offset + 21],
                ]) as usize;

                let data_start = offset + content_offset;
                let data_end = data_start + content_size;

                if data_end <= mft_record.len() && content_size > 0 {
                    let actual_size = content_size.min(expected_size as usize);
                    return Some(mft_record[data_start..data_start + actual_size].to_vec());
                }
            }
        }

        offset += attr_length;
    }

    None
}

/// Get priority for file paths (lower = higher priority)
/// Prioritizes user folders where recent activity happens
fn get_path_priority(path: &str) -> u32 {
    let path_lower = path.to_lowercase();
    
    // High priority folders (most likely to have recently deleted files)
    if path_lower.contains("\\desktop\\") || path_lower.ends_with("\\desktop") {
        return 1;
    }
    if path_lower.contains("\\downloads\\") {
        return 2;
    }
    if path_lower.contains("\\documents\\") {
        return 3;
    }
    if path_lower.contains("\\pictures\\") || path_lower.contains("\\videos\\") {
        return 4;
    }
    if path_lower.contains("\\music\\") {
        return 5;
    }
    
    // User profile folders
    if path_lower.contains("\\users\\") {
        return 10;
    }
    
    // Everything else
    99
}

/// Resolve a file path by directly reading parent MFT records
/// Used as fallback when dir_map doesn't have the parent directory
fn resolve_path_from_mft(
    reader: &mut crate::filesystem_disk_reader::FileSystemDiskReader,
    drive_letter: &str,
    parent_record: u64,
    file_name: &str,
) -> String {
    let mut path_parts: Vec<String> = vec![file_name.to_string()];
    let mut current = parent_record;
    let mut depth = 0;
    
    while current != 5 && depth < 50 {
        match reader.read_mft_record(current) {
            Ok(buffer) => {
                if let Some(entry) = parse_mft_record(&buffer, current) {
                    if !entry.file_name.starts_with('$') && !entry.file_name.is_empty() && entry.file_name != "." {
                        path_parts.push(entry.file_name.clone());
                    }
                    if entry.parent_record == current {
                        break; // Self-referencing, stop
                    }
                    current = entry.parent_record;
                    depth += 1;
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    
    path_parts.reverse();
    format!("{}:\\{}", drive_letter, path_parts.join("\\"))
}

/// Build full path by traversing parent references
fn build_full_path(
    drive_letter: &str, 
    parent_record: u64, 
    file_name: &str, 
    dir_map: &std::collections::HashMap<u64, (u64, String)>
) -> String {
    let mut path_parts: Vec<String> = vec![file_name.to_string()];
    let mut current_parent = parent_record;
    let mut depth = 0;
    
    // Root directory's parent reference is usually 5 (itself)
    while current_parent != 5 && depth < 100 {
        if let Some((next_parent, dir_name)) = dir_map.get(&current_parent) {
            // Skip system directories like "." 
            if !dir_name.starts_with('$') && !dir_name.is_empty() && dir_name != "." {
                path_parts.push(dir_name.clone());
            }
            current_parent = *next_parent;
            depth += 1;
        } else {
            break;
        }
    }
    
    // Reverse to get path from root to file
    path_parts.reverse();
    format!("{}:\\{}", drive_letter, path_parts.join("\\"))
}

/// Check if file is a temporary file that should be filtered out
fn is_temp_file(name: &str, path: &str) -> bool {
    let name_lower = name.to_lowercase();
    let path_lower = path.to_lowercase();
    
    // Temp file extensions
    if name_lower.ends_with(".tmp") || 
       name_lower.ends_with(".temp") ||
       name_lower.ends_with(".bak") ||
       name_lower.ends_with(".~") ||
       name_lower.ends_with(".lock") ||
       name_lower.ends_with(".partial") ||
       name_lower.ends_with(".crdownload") ||
       name_lower.ends_with(".part") {
        return true;
    }
    
    // Temp filename patterns
    if name_lower.starts_with("~$") ||  // Office temp files
       name_lower.starts_with("~") ||   // General temp prefix
       name_lower.starts_with("tmp") ||
       name_lower.starts_with("temp") ||
       name_lower.contains(".tmp") {
        return true;
    }
    
    // Temp directories
    if path_lower.contains("\\temp\\") ||
       path_lower.contains("\\tmp\\") ||
       path_lower.contains("\\appdata\\local\\temp") ||
       path_lower.contains("\\windows\\temp") ||
       path_lower.contains("\\$recycle.bin") ||
       path_lower.contains("\\system volume information") ||
       path_lower.contains("\\prefetch") ||
       path_lower.contains("\\.cache\\") ||
       path_lower.contains("\\cache\\") ||
       path_lower.contains("\\thumbnails\\") ||
       path_lower.contains("\\winsxs\\") {
        return true;
    }
    
    false
}

/// Convert MFT entry to recoverable file with proper path resolution
fn mft_entry_to_recoverable_with_path(
    drive_letter: &str, 
    entry: &MftEntry,
    dir_map: &std::collections::HashMap<u64, (u64, String)>
) -> Option<RecoverableFileFS> {
    // Skip entries with empty names (invalid MFT records)
    if entry.file_name.is_empty() {
        return None;
    }
    
    // Skip system files and directories
    if entry.is_directory || entry.file_name.starts_with('$') {
        return None;
    }
    
    // Determine file category
    let extension = if let Some(pos) = entry.file_name.rfind('.') {
        entry.file_name[pos + 1..].to_lowercase()
    } else {
        String::new()
    };
    
    // Filter out Windows shortcuts and temporary files - not useful for recovery
    match extension.as_str() {
        "lnk" | "url" | "ini" | "tmp" | "temp" | "log" | "bak" | "thumbs" => return None,
        _ => {}
    }
    
    // Filter out 0-byte files - nothing to recover
    if entry.file_size == 0 {
        return None;
    }
    
    let category = categorize_file(&extension);
    let file_type = extension.clone();
    
    // Recovery chance based on deletion status, size, and data availability
    let recovery_chance = if !entry.is_deleted {
        95 // Existing files: very high chance
    } else if entry.data_runs.is_empty() {
        // Deleted with no cluster data
        if entry.file_size <= 1024 {
            85 // Small files might be resident in MFT
        } else if entry.file_size <= 100 * 1024 {
            60 // Small-medium files might be in Recycle Bin or VSS
        } else {
            15 // Larger files without cluster data are very unlikely to recover
        }
    } else {
        // Deleted with cluster data available
        if entry.file_size <= 1024 * 1024 {
            85 // Small deleted files with data_runs: high chance
        } else {
            70 // Large deleted files: moderate-high chance
        }
    };
    
    // Filter out files that are deleted with no data AND very small (those are likely
    // empty temporary MFT scraps, not real user files). BUT keep larger deleted files
    // even without data_runs — they represent real files the user deleted.
    if entry.is_deleted && entry.data_runs.is_empty() && entry.file_size == 0 {
        return None;
    }
    
    // Format timestamps
    let modified = format_timestamp(entry.modified_time);
    let created = format_timestamp(entry.created_time);
    
    // Serialize data runs to JSON
    let data_runs_json = serde_json::to_string(&entry.data_runs)
        .unwrap_or_else(|_| "[]".to_string());
    
    // Get first cluster offset if available
    let cluster_offset = entry.data_runs.first().map(|r| r.cluster_offset);
    
    // Build proper full path using directory map
    let full_path = build_full_path(drive_letter, entry.parent_record, &entry.file_name, dir_map);
    
    // TEMPORARILY DISABLED: Don't filter temp files for debugging
    // Skip temporary files
    // if is_temp_file(&entry.file_name, &full_path) {
    //     if extension == "png" {
    //         eprintln!("PNG FILTERED: temp file\n");
    //     }
    //     return None;
    // }

    // TEMPORARILY DISABLED: Show all deleted files regardless of cluster data
    // For deleted files: be lenient with filtering
    // if entry.is_deleted {
    //     if entry.data_runs.is_empty() && entry.file_size > 10 * 1024 * 1024 {
    //         if extension == "png" {
    //             eprintln!("PNG FILTERED: large deleted file with no cluster data\n");
    //         }
    //         return None;
    //     }
    //     if extension == "png" {
    //         eprintln!("PNG INCLUDED: deleted file passed filters\n");
    //     }
    // }

    Some(RecoverableFileFS {
        id: format!("fs_mft_{}", entry.record_number),
        name: entry.file_name.clone(),
        path: full_path,
        size: entry.file_size,
        extension,
        category,
        file_type,
        modified,
        created,
        is_deleted: entry.is_deleted,
        recovery_chance,
        source: "mft_filesystem".to_string(),
        cluster_offset,
        data_runs: Some(data_runs_json),
    })
}

/// Categorize file by extension
fn categorize_file(extension: &str) -> String {
    match extension {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp" | "svg" => "Images".to_string(),
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => "Videos".to_string(),
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => "Audio".to_string(),
        "pdf" => "PDF Documents".to_string(),
        "doc" | "docx" | "odt" | "rtf" => "Word Documents".to_string(),
        "xls" | "xlsx" | "ods" | "csv" => "Spreadsheets".to_string(),
        "ppt" | "pptx" | "odp" => "Presentations".to_string(),
        "txt" | "log" | "md" | "json" | "xml" | "html" | "css" | "js" => "Text Files".to_string(),
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" => "Archives".to_string(),
        "exe" | "dll" | "sys" | "msi" => "Executables".to_string(),
        _ => "Other".to_string(),
    }
}

/// Format timestamp from i64 to string
fn format_timestamp(unix_ts: i64) -> String {
    if unix_ts <= 0 {
        return "Unknown".to_string();
    }
    
    chrono::DateTime::from_timestamp(unix_ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}
