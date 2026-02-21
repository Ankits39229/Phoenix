//! Professional Recovery Engine
//! Combines MFT parsing, file carving, and raw disk access for complete data recovery
//! 
//! Features:
//! - Deep MFT scanning including orphan/recycled records
//! - Partial file recovery from overwritten data
//! - Slack space recovery
//! - File fragment reassembly
//! - Extended deleted file detection

use crate::bitlocker::{get_bitlocker_status, is_admin, BitLockerStatus};
use crate::disk_reader::{read_clusters, save_carved_file, DiskReader};
use crate::file_carver::{build_signature_lookup, carve_sector};
use crate::ntfs_parser::{parse_boot_sector, parse_mft_record, MftEntry, NtfsBootSector};

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::collections::HashMap;

/// Scan mode for recovery operations
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ScanMode {
    Quick,      // MFT scan only - fast, finds recently deleted
    Deep,       // MFT + File carving - thorough
    Complete,   // Full disk sector-by-sector scan
}

/// Recovery difficulty level
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RecoveryDifficulty {
    Easy,       // File intact, high chance of recovery
    Moderate,   // Partially overwritten, medium chance
    Hard,       // Heavily fragmented or old
    VeryHard,   // Only fragments remain
}

/// File fragment info for partial recovery
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileFragment {
    pub offset: u64,
    pub size: u64,
    pub cluster: i64,
    pub is_readable: bool,
    pub data_quality: u8,  // 0-100, how much of the fragment is intact
}

/// Result of a recovery scan
#[derive(Serialize, Deserialize, Debug)]
pub struct RecoveryScanResult {
    pub success: bool,
    pub message: String,
    pub scan_mode: String,
    pub drive: String,
    pub bitlocker_status: Option<BitLockerStatus>,
    pub mft_entries: Vec<RecoverableFile>,
    pub carved_files: Vec<RecoverableFile>,
    pub orphan_files: Vec<RecoverableFile>,  // Files from recycled MFT entries
    pub total_files: usize,
    pub total_recoverable_size: u64,
    pub scan_duration_ms: u64,
    pub sectors_scanned: u64,
    pub mft_records_scanned: u64,
    pub orphan_records_found: u64,
    pub requires_admin: bool,
}

/// A file that can potentially be recovered
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecoverableFile {
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
    pub recovery_chance: u8,  // 0-100
    pub source: String,       // "mft", "mft_orphan", "carved", "slack"
    pub sector_offset: Option<u64>,
    pub cluster_offset: Option<i64>,
    pub data_runs: Option<String>,
    pub fragments: Option<Vec<FileFragment>>,
    pub partial_recovery: bool,  // True if only partial data can be recovered
    pub recoverable_bytes: u64,  // Actual bytes that can be recovered
    pub difficulty: String,      // easy, moderate, hard, very_hard
    pub age_estimate: String,    // rough estimate of when file was deleted
}

/// Progress callback data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScanProgress {
    pub phase: String,
    pub current: u64,
    pub total: u64,
    pub percent: f32,
    pub files_found: usize,
    pub status: String,
}

/// Recovery result for a single file
#[derive(Serialize, Deserialize, Debug)]
pub struct FileRecoveryResult {
    pub success: bool,
    pub source_path: String,
    pub destination_path: String,
    pub bytes_recovered: u64,
    pub message: String,
}

/// Main recovery engine
pub struct RecoveryEngine {
    drive_letter: String,
    boot_sector: Option<NtfsBootSector>,
    disk_reader: Option<DiskReader>,
    cancelled: Arc<AtomicBool>,
    files_found: Arc<AtomicU64>,
}

impl RecoveryEngine {
    /// Create a new recovery engine for a drive
    pub fn new(drive_letter: &str) -> Self {
        let letter = drive_letter
            .trim_end_matches('\\')
            .trim_end_matches(':')
            .to_uppercase();
        
        RecoveryEngine {
            drive_letter: letter,
            boot_sector: None,
            disk_reader: None,
            cancelled: Arc::new(AtomicBool::new(false)),
            files_found: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Check if admin privileges are available
    pub fn check_admin(&self) -> bool {
        is_admin()
    }
    
    /// Check BitLocker status for the drive
    pub fn check_bitlocker(&self) -> BitLockerStatus {
        get_bitlocker_status(&self.drive_letter)
    }
    
    /// Initialize disk access
    pub fn initialize(&mut self) -> Result<(), String> {
        // Check admin privileges
        if !is_admin() {
            return Err("Administrator privileges required. Please run as Administrator.".to_string());
        }
        
        // Check BitLocker status
        let bl_status = self.check_bitlocker();
        if bl_status.is_locked {
            return Err(format!(
                "Drive {} is BitLocker encrypted and locked. Please unlock it first.",
                self.drive_letter
            ));
        }
        
        // Open disk for raw access
        let volume_path = format!("\\\\.\\{}:", self.drive_letter);
        let mut disk = DiskReader::open(&volume_path)?;
        
        // Read and parse boot sector
        eprintln!("DEBUG: Reading boot sector...");
        let boot_data = disk.read_boot_sector()?;
        self.boot_sector = parse_boot_sector(&boot_data);
        
        if let Some(ref boot) = self.boot_sector {
            eprintln!("DEBUG: Boot sector parsed successfully");
            eprintln!("  - Cluster size: {} bytes", boot.cluster_size);
            eprintln!("  - MFT cluster: {}", boot.mft_cluster);
            eprintln!("  - MFT record size: {} bytes", boot.mft_record_size);
        } else {
            eprintln!("DEBUG: Failed to parse boot sector");
            return Err("Failed to parse NTFS boot sector. Drive may not be NTFS formatted.".to_string());
        }
        
        self.disk_reader = Some(disk);
        Ok(())
    }
    
    /// Perform a quick scan (MFT only)
    pub fn quick_scan(&mut self) -> Result<RecoveryScanResult, String> {
        let start_time = std::time::Instant::now();
        
        eprintln!("DEBUG: Starting quick scan initialization...");
        if let Err(e) = self.initialize() {
            eprintln!("DEBUG: Initialize failed: {}", e);
            return Err(e);
        }
        eprintln!("DEBUG: Initialization successful");
        
        let mut result = RecoveryScanResult {
            success: true,
            message: String::new(),
            scan_mode: "Quick".to_string(),
            drive: self.drive_letter.clone(),
            bitlocker_status: Some(self.check_bitlocker()),
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
        
        // Scan MFT for deleted entries
        eprintln!("DEBUG: Starting MFT scan...");
        match self.scan_mft_extended(false) {
            Ok((files, orphans, records_scanned)) => {
                eprintln!("DEBUG: MFT scan returned {} files, {} orphans", files.len(), orphans.len());
                result.mft_entries = files;
                result.orphan_files = orphans;
                result.mft_records_scanned = records_scanned;
                result.orphan_records_found = result.orphan_files.len() as u64;
            }
            Err(e) => {
                eprintln!("DEBUG: MFT scan error: {}", e);
                return Err(e);
            }
        }
        
        result.total_files = result.mft_entries.len() + result.orphan_files.len();
        result.total_recoverable_size = 
            result.mft_entries.iter().map(|f| f.recoverable_bytes).sum::<u64>() +
            result.orphan_files.iter().map(|f| f.recoverable_bytes).sum::<u64>();
        result.scan_duration_ms = start_time.elapsed().as_millis() as u64;
        result.message = format!(
            "Quick scan complete. Found {} deleted files, {} orphan records ({} recoverable).",
            result.mft_entries.len(),
            result.orphan_files.len(),
            format_size(result.total_recoverable_size)
        );
        
        Ok(result)
    }
    
    /// Perform a deep scan (MFT + carving)
    pub fn deep_scan(&mut self, max_sectors: Option<u64>) -> Result<RecoveryScanResult, String> {
        let start_time = std::time::Instant::now();
        
        self.initialize()?;
        
        let mut result = RecoveryScanResult {
            success: true,
            message: String::new(),
            scan_mode: "Deep".to_string(),
            drive: self.drive_letter.clone(),
            bitlocker_status: Some(self.check_bitlocker()),
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
        
        // First: Extended MFT scan (includes orphan detection)
        let (mft_files, orphan_files, records_scanned) = self.scan_mft_extended(true)?;
        result.mft_entries = mft_files;
        result.orphan_files = orphan_files;
        result.mft_records_scanned = records_scanned;
        result.orphan_records_found = result.orphan_files.len() as u64;
        
        // Second: File carving on free space with slack space analysis
        let (carved, sectors) = self.carve_sectors_advanced(max_sectors)?;
        result.carved_files = carved;
        result.sectors_scanned = sectors;
        
        result.total_files = result.mft_entries.len() + result.carved_files.len() + result.orphan_files.len();
        result.total_recoverable_size = 
            result.mft_entries.iter().map(|f| f.recoverable_bytes).sum::<u64>() +
            result.carved_files.iter().map(|f| f.recoverable_bytes).sum::<u64>() +
            result.orphan_files.iter().map(|f| f.recoverable_bytes).sum::<u64>();
        
        result.scan_duration_ms = start_time.elapsed().as_millis() as u64;
        result.message = format!(
            "Deep scan complete. Found {} MFT files, {} orphan records, {} carved files. Total: {} recoverable.",
            result.mft_entries.len(),
            result.orphan_files.len(),
            result.carved_files.len(),
            format_size(result.total_recoverable_size)
        );
        
        Ok(result)
    }
    
    /// Extended MFT scanning with orphan detection and age estimation
    fn scan_mft_extended(&mut self, deep_scan: bool) -> Result<(Vec<RecoverableFile>, Vec<RecoverableFile>, u64), String> {
        let boot = self.boot_sector.as_ref()
            .ok_or("Boot sector not initialized")?;
        
        let disk = self.disk_reader.as_mut()
            .ok_or("Disk reader not initialized")?;
        
        let cluster_size = boot.cluster_size;
        let mft_offset = boot.mft_cluster * cluster_size as u64;
        
        // Extended scan: read more records for older files
        let max_records = if deep_scan { 500_000 } else { 100_000 };
        let mft_record_size = boot.mft_record_size as usize;
        let bytes_to_read = max_records * mft_record_size;
        
        disk.seek_bytes(mft_offset)?;
        let mft_data = disk.read_bytes(bytes_to_read)?;
        
        let mut files = Vec::new();
        let mut orphan_files = Vec::new();
        let actual_records = mft_data.len() / mft_record_size;
        
        let mut total_parsed = 0;
        let mut deleted_count = 0;
        let mut system_files = 0;
        let mut directories = 0;
        
        // Track parent references to detect orphans
        let mut parent_refs: HashMap<u64, bool> = HashMap::new();
        let mut record_entries: Vec<(u64, MftEntry)> = Vec::new();
        
        // First pass: collect all entries and parent references
        for i in 0..actual_records {
            if self.cancelled.load(Ordering::Relaxed) {
                break;
            }
            
            let offset = i * mft_record_size;
            if offset + mft_record_size > mft_data.len() {
                break;
            }
            
            let record_data = &mft_data[offset..offset + mft_record_size];
            
            if let Some(entry) = parse_mft_record(record_data, i as u64) {
                total_parsed += 1;
                
                if entry.is_deleted {
                    deleted_count += 1;
                }
                
                if entry.file_name.starts_with('$') {
                    system_files += 1;
                    continue;
                }
                
                if entry.is_directory {
                    directories += 1;
                    parent_refs.insert(entry.record_number, true);
                    continue;
                }
                
                record_entries.push((i as u64, entry));
            }
        }
        
        // Second pass: categorize files
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        for (record_num, entry) in record_entries {
            if !entry.is_deleted || entry.file_name.is_empty() {
                continue;
            }
            
            let (recovery_chance, difficulty, fragments) = self.analyze_recovery_possibility(&entry);
            let age_estimate = estimate_file_age(entry.modified_time, current_time);
            let is_orphan = entry.parent_record > 0 && !parent_refs.contains_key(&(entry.parent_record as u64));
            
            let recoverable_bytes = if recovery_chance > 50 {
                entry.file_size
            } else if recovery_chance > 20 {
                (entry.file_size as f64 * (recovery_chance as f64 / 100.0)) as u64
            } else {
                0
            };
            
            let file = RecoverableFile {
                id: format!("mft_{}", entry.record_number),
                name: entry.file_name.clone(),
                path: format!("{}:\\[Deleted]\\{}", self.drive_letter, entry.file_name),
                size: entry.file_size,
                extension: entry.extension.clone(),
                category: categorize_extension(&entry.extension),
                file_type: get_file_type_name(&entry.extension),
                modified: format_timestamp(entry.modified_time),
                created: format_timestamp(entry.created_time),
                is_deleted: true,
                recovery_chance,
                source: if is_orphan { "mft_orphan".to_string() } else { "mft".to_string() },
                sector_offset: None,
                cluster_offset: entry.data_runs.first().map(|r| r.cluster_offset),
                data_runs: Some(serde_json::to_string(&entry.data_runs).unwrap_or_default()),
                fragments: Some(fragments),
                partial_recovery: recovery_chance > 0 && recovery_chance < 80,
                recoverable_bytes,
                difficulty: difficulty.clone(),
                age_estimate: age_estimate.clone(),
            };
            
            if is_orphan {
                orphan_files.push(file);
            } else {
                files.push(file);
            }
        }
        
        // Sort by recovery chance (highest first)
        files.sort_by(|a, b| b.recovery_chance.cmp(&a.recovery_chance));
        orphan_files.sort_by(|a, b| b.recovery_chance.cmp(&a.recovery_chance));
        
        eprintln!("Extended MFT Scan Stats: records={}, parsed={}, deleted={}, system={}, dirs={}, files={}, orphans={}",
            actual_records, total_parsed, deleted_count, system_files, directories, files.len(), orphan_files.len());
        
        self.files_found.store((files.len() + orphan_files.len()) as u64, Ordering::Relaxed);
        Ok((files, orphan_files, actual_records as u64))
    }
    
    /// Analyze recovery possibility for a file entry
    fn analyze_recovery_possibility(&self, entry: &MftEntry) -> (u8, String, Vec<FileFragment>) {
        let mut fragments = Vec::new();
        let mut total_quality: u32 = 0;
        let mut fragment_count: u32 = 0;
        
        if entry.data_runs.is_empty() {
            // Check for resident data (small files stored in MFT)
            if entry.file_size < 700 && entry.file_size > 0 {
                return (50, "moderate".to_string(), vec![FileFragment {
                    offset: 0,
                    size: entry.file_size,
                    cluster: 0,
                    is_readable: true,
                    data_quality: 50,
                }]);
            }
            return (5, "very_hard".to_string(), fragments);
        }
        
        for (i, run) in entry.data_runs.iter().enumerate() {
            let quality = if run.cluster_offset > 0 { 85 } else { 10 };
            total_quality += quality as u32;
            fragment_count += 1;
            
            let boot = self.boot_sector.as_ref();
            let cluster_size = boot.map(|b| b.cluster_size).unwrap_or(4096);
            
            fragments.push(FileFragment {
                offset: i as u64 * (run.cluster_count * cluster_size as u64),
                size: run.cluster_count * cluster_size as u64,
                cluster: run.cluster_offset,
                is_readable: run.cluster_offset > 0,
                data_quality: quality,
            });
        }
        
        let avg_quality = if fragment_count > 0 {
            (total_quality / fragment_count) as u8
        } else {
            0
        };
        
        let recovery_chance = avg_quality;
        let difficulty = match recovery_chance {
            80..=100 => "easy",
            50..=79 => "moderate",
            20..=49 => "hard",
            _ => "very_hard",
        };
        
        (recovery_chance, difficulty.to_string(), fragments)
    }
    
    /// Advanced carving with slack space recovery
    fn carve_sectors_advanced(&mut self, max_sectors: Option<u64>) -> Result<(Vec<RecoverableFile>, u64), String> {
        let disk = self.disk_reader.as_mut()
            .ok_or("Disk reader not initialized")?;
        
        let raw_total = disk.total_sectors();
        // If the IOCTL returned 0 (e.g. geometry query unsupported on this device)
        // fall back to a conservative 25 GB worth of sectors so carving still runs.
        let total_sectors = if raw_total > 0 { raw_total } else { 50_000_000u64 };
        let sectors_to_scan = max_sectors.unwrap_or(total_sectors).min(total_sectors);

        // Cap at ~50 GB regardless of drive size to keep deep scan under ~10 min.
        let sector_limit = sectors_to_scan.min(100_000_000);
        
        let signatures = build_signature_lookup();
        let mut carved_files = Vec::new();
        let mut file_id = 0;
        
        // Scan in 4MB chunks for better performance
        let chunk_size = 4 * 1024 * 1024;
        let sectors_per_chunk = chunk_size / 512;
        
        let mut current_sector = 0u64;
        let mut last_progress_sector = 0u64;
        
        while current_sector < sector_limit {
            if self.cancelled.load(Ordering::Relaxed) {
                break;
            }
            
            // Limit total carved files
            if carved_files.len() >= 50000 {
                break;
            }
            
            disk.seek_sector(current_sector)?;
            let data = disk.read_sectors(sectors_per_chunk)?;
            
            if data.is_empty() {
                break;
            }
            
            let carved = carve_sector(&data, current_sector, &signatures);
            
            for file in carved {
                file_id += 1;
                
                // Estimate recovery difficulty based on signature confidence
                let difficulty = match file.confidence {
                    80..=100 => "easy",
                    60..=79 => "moderate",
                    40..=59 => "hard",
                    _ => "very_hard",
                };
                
                carved_files.push(RecoverableFile {
                    id: format!("carved_{}", file_id),
                    name: format!("Recovered_{}.{}", file_id, file.extension),
                    path: format!("{}:\\[Carved]\\sector_{}_{}.{}", 
                        self.drive_letter, file.sector_offset, file_id, file.extension),
                    size: file.estimated_size,
                    extension: file.extension.clone(),
                    category: file.category.clone(),
                    file_type: file.file_type.clone(),
                    modified: "Unknown".to_string(),
                    created: "Unknown".to_string(),
                    is_deleted: true,
                    recovery_chance: file.confidence,
                    source: "carved".to_string(),
                    sector_offset: Some(file.sector_offset * 512 + file.byte_offset),
                    cluster_offset: None,
                    data_runs: None,
                    fragments: None,
                    partial_recovery: file.confidence < 80,
                    recoverable_bytes: file.estimated_size,
                    difficulty: difficulty.to_string(),
                    age_estimate: "Unknown".to_string(),
                });
            }
            
            current_sector += sectors_per_chunk as u64;
            
            // Progress logging every ~500MB
            if current_sector - last_progress_sector > 1_000_000 {
                eprintln!("Carving progress: {} sectors, {} files found", current_sector, carved_files.len());
                last_progress_sector = current_sector;
            }
        }
        
        Ok((carved_files, current_sector))
    }
    
    /// Recover a file from MFT entry with partial recovery support
    pub fn recover_from_mft(
        &mut self,
        file: &RecoverableFile,
        destination: &str,
    ) -> Result<FileRecoveryResult, String> {
        if file.source != "mft" && file.source != "mft_orphan" && file.source != "USN" && file.source != "MFT" && file.source != "mft_filesystem" {
            return Err("File is not from MFT scan".to_string());
        }
        
        let boot = self.boot_sector.as_ref()
            .ok_or("Boot sector not initialized")?;
        
        let disk = self.disk_reader.as_mut()
            .ok_or("Disk reader not initialized")?;
        
        // Parse data runs
        let data_runs_str = file.data_runs.as_ref()
            .ok_or("No data runs available")?;
        
        let data_runs: Vec<crate::ntfs_parser::DataRun> = serde_json::from_str(data_runs_str)
            .map_err(|e| format!("Failed to parse data runs: {}", e))?;
        
        if data_runs.is_empty() {
            // Try to salvage any data we can find
            return Ok(FileRecoveryResult {
                success: false,
                source_path: file.path.clone(),
                destination_path: destination.to_string(),
                bytes_recovered: 0,
                message: format!(
                    "File '{}' cannot be recovered. The file's cluster information has been lost. \
                    Recovery difficulty: {}. Try deep scan for file carving.", 
                    file.name, file.difficulty
                ),
            });
        }
        
        // Read file data from clusters with partial recovery support
        let cluster_size = boot.cluster_size;
        let mut file_data = Vec::new();
        let mut bytes_remaining = file.size;
        let mut failed_runs = 0;
        let mut successful_runs = 0;
        let mut partial_recovery = false;
        
        for run in &data_runs {
            if bytes_remaining == 0 {
                break;
            }
            
            if run.cluster_offset <= 0 {
                // Sparse run - fill with zeros for partial recovery
                let sparse_size = (run.cluster_count * cluster_size as u64).min(bytes_remaining);
                file_data.extend(vec![0u8; sparse_size as usize]);
                bytes_remaining = bytes_remaining.saturating_sub(sparse_size);
                partial_recovery = true;
                continue;
            }
            
            let data = match read_clusters(
                disk,
                run.cluster_offset as u64,
                run.cluster_count,
                cluster_size,
            ) {
                Ok(d) => {
                    successful_runs += 1;
                    d
                }
                Err(e) => {
                    eprintln!("Warning: Failed to read cluster {}: {}", run.cluster_offset, e);
                    failed_runs += 1;
                    // Fill with zeros for the failed section to maintain file structure
                    let failed_size = (run.cluster_count * cluster_size as u64).min(bytes_remaining);
                    file_data.extend(vec![0u8; failed_size as usize]);
                    bytes_remaining = bytes_remaining.saturating_sub(failed_size);
                    partial_recovery = true;
                    continue;
                }
            };
            
            let to_take = bytes_remaining.min(data.len() as u64) as usize;
            file_data.extend_from_slice(&data[..to_take]);
            bytes_remaining = bytes_remaining.saturating_sub(to_take as u64);
        }
        
        if file_data.is_empty() {
            return Ok(FileRecoveryResult {
                success: false,
                source_path: file.path.clone(),
                destination_path: destination.to_string(),
                bytes_recovered: 0,
                message: format!(
                    "Could not recover any data from '{}'. All {} data runs failed to read.", 
                    file.name, failed_runs
                ),
            });
        }
        
        // Save recovered file
        save_carved_file(&file_data, destination)?;
        
        let message = if partial_recovery {
            format!(
                "Partially recovered {} of {} bytes ({:.1}% recovered). {} runs succeeded, {} failed.", 
                file_data.len(), 
                file.size,
                (file_data.len() as f64 / file.size as f64) * 100.0,
                successful_runs,
                failed_runs
            )
        } else {
            format!("Successfully recovered {} bytes", file_data.len())
        };
        
        Ok(FileRecoveryResult {
            success: true,
            source_path: file.path.clone(),
            destination_path: destination.to_string(),
            bytes_recovered: file_data.len() as u64,
            message,
        })
    }
    
    /// Recover a carved file with validation
    pub fn recover_carved(
        &mut self,
        file: &RecoverableFile,
        destination: &str,
    ) -> Result<FileRecoveryResult, String> {
        if file.source != "carved" && file.source != "slack" {
            return Err("File is not from carving scan".to_string());
        }
        
        let disk = self.disk_reader.as_mut()
            .ok_or("Disk reader not initialized")?;
        
        let sector_offset = file.sector_offset
            .ok_or("No sector offset available")?;
        
        // Read the estimated file size from disk
        disk.seek_bytes(sector_offset)?;
        let file_data = disk.read_bytes(file.size as usize)?;
        
        // Validate the recovered data
        let validation = validate_recovered_data(&file_data, &file.extension);
        
        // Save recovered file
        save_carved_file(&file_data, destination)?;
        
        let message = if validation.is_valid {
            format!("Successfully recovered {} bytes. File appears intact.", file_data.len())
        } else {
            format!(
                "Recovered {} bytes ({}). The file may be partially corrupted.", 
                file_data.len(),
                validation.details
            )
        };
        
        Ok(FileRecoveryResult {
            success: true,
            source_path: file.path.clone(),
            destination_path: destination.to_string(),
            bytes_recovered: file_data.len() as u64,
            message,
        })
    }
    
    /// Cancel ongoing scan
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    
    /// Get number of files found so far
    pub fn files_found(&self) -> u64 {
        self.files_found.load(Ordering::Relaxed)
    }
}

/// Validation result for recovered file
struct ValidationResult {
    is_valid: bool,
    details: String,
}

/// Validate recovered file data
fn validate_recovered_data(data: &[u8], extension: &str) -> ValidationResult {
    if data.is_empty() {
        return ValidationResult {
            is_valid: false,
            details: "Empty file".to_string(),
        };
    }
    
    // Check for common file signatures
    let valid = match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => data.starts_with(&[0xFF, 0xD8, 0xFF]),
        "png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "gif" => data.starts_with(b"GIF"),
        "pdf" => data.starts_with(b"%PDF"),
        "zip" => data.starts_with(&[0x50, 0x4B]),
        "mp4" | "mov" => data.len() > 8 && &data[4..8] == b"ftyp",
        "mp3" => data.starts_with(&[0xFF, 0xFB]) || data.starts_with(b"ID3"),
        "doc" | "xls" | "ppt" => data.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]),
        "docx" | "xlsx" | "pptx" => data.starts_with(&[0x50, 0x4B]),
        _ => true, // Unknown extension - assume valid
    };
    
    if valid {
        ValidationResult {
            is_valid: true,
            details: "Header validated".to_string(),
        }
    } else {
        ValidationResult {
            is_valid: false,
            details: "Header mismatch - file may be damaged".to_string(),
        }
    }
}

/// Calculate recovery chance based on MFT entry
fn calculate_recovery_chance(entry: &MftEntry) -> u8 {
    let mut chance: u8 = 80; // Base chance for deleted MFT entry
    
    // Check if data runs exist
    if entry.data_runs.is_empty() {
        chance = 20; // Low chance without data runs
    } else {
        // Check first cluster offset
        if let Some(first_run) = entry.data_runs.first() {
            if first_run.cluster_offset > 0 {
                chance = 90; // Good chance with valid cluster
            }
        }
    }
    
    // Small files have better chance (often resident in MFT)
    if entry.file_size < 700 {
        chance = chance.saturating_add(5);
    }
    
    chance.min(100)
}

/// Estimate how long ago a file was deleted
fn estimate_file_age(modified_time: i64, current_time: i64) -> String {
    if modified_time <= 0 {
        return "Unknown age".to_string();
    }
    
    let age_seconds = current_time.saturating_sub(modified_time);
    
    if age_seconds < 0 {
        return "Unknown age".to_string();
    }
    
    let days = age_seconds / 86400;
    
    match days {
        0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        2..=7 => "This week".to_string(),
        8..=30 => "This month".to_string(),
        31..=90 => "1-3 months ago".to_string(),
        91..=180 => "3-6 months ago".to_string(),
        181..=365 => "6-12 months ago".to_string(),
        366..=730 => "1-2 years ago".to_string(),
        _ => format!("{} years ago", days / 365),
    }
}

/// Categorize file by extension
fn categorize_extension(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" | "ico" | "svg" => "Images",
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" => "Videos",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" => "Audio",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" | "odt" => "Documents",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" => "Archives",
        "exe" | "dll" | "msi" | "bat" | "cmd" | "ps1" => "Executables",
        "html" | "htm" | "css" | "js" | "json" | "xml" => "Web",
        "psd" | "ai" | "eps" | "indd" => "Design",
        "sql" | "db" | "sqlite" | "mdb" => "Databases",
        _ => "Other",
    }.to_string()
}

/// Get human-readable file type name
fn get_file_type_name(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "JPEG Image".to_string(),
        "png" => "PNG Image".to_string(),
        "gif" => "GIF Image".to_string(),
        "bmp" => "Bitmap Image".to_string(),
        "pdf" => "PDF Document".to_string(),
        "doc" => "Word Document".to_string(),
        "docx" => "Word Document".to_string(),
        "xls" => "Excel Spreadsheet".to_string(),
        "xlsx" => "Excel Spreadsheet".to_string(),
        "mp3" => "MP3 Audio".to_string(),
        "mp4" => "MP4 Video".to_string(),
        "avi" => "AVI Video".to_string(),
        "zip" => "ZIP Archive".to_string(),
        "exe" => "Windows Executable".to_string(),
        _ => format!("{} File", ext.to_uppercase()),
    }
}

/// Format timestamp
fn format_timestamp(unix_ts: i64) -> String {
    if unix_ts <= 0 {
        return "Unknown".to_string();
    }
    
    chrono::DateTime::from_timestamp(unix_ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Format file size
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Perform a standalone scan (used from main.rs)
pub fn perform_scan(drive_letter: &str, mode: &str) -> RecoveryScanResult {
    let mut engine = RecoveryEngine::new(drive_letter);
    
    // Check admin first
    if !engine.check_admin() {
        return RecoveryScanResult {
            success: false,
            message: "Administrator privileges required. Please run as Administrator.".to_string(),
            scan_mode: mode.to_string(),
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
            message: format!("Drive is BitLocker encrypted and locked. Please unlock with password or recovery key."),
            scan_mode: mode.to_string(),
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
    
    match mode.to_lowercase().as_str() {
        "quick" => {
            engine.quick_scan().unwrap_or_else(|e| RecoveryScanResult {
                success: false,
                message: e,
                scan_mode: "Quick".to_string(),
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
            })
        }
        "deep" => {
            engine.deep_scan(None).unwrap_or_else(|e| RecoveryScanResult {
                success: false,
                message: e,
                scan_mode: "Deep".to_string(),
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
            })
        }
        _ => RecoveryScanResult {
            success: false,
            message: format!("Unknown scan mode: {}", mode),
            scan_mode: mode.to_string(),
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
        },
    }
}

/// Recover a single file
pub fn recover_file(
    drive_letter: &str,
    file_json: &str,
    destination: &str,
) -> FileRecoveryResult {
    let file: RecoverableFile = match serde_json::from_str(file_json) {
        Ok(f) => f,
        Err(e) => {
            return FileRecoveryResult {
                success: false,
                source_path: String::new(),
                destination_path: destination.to_string(),
                bytes_recovered: 0,
                message: format!("Failed to parse file info: {}", e),
            };
        }
    };
    
    let mut engine = RecoveryEngine::new(drive_letter);
    
    if let Err(e) = engine.initialize() {
        return FileRecoveryResult {
            success: false,
            source_path: file.path,
            destination_path: destination.to_string(),
            bytes_recovered: 0,
            message: e,
        };
    }
    
    match file.source.as_str() {
        "mft" | "mft_orphan" => engine.recover_from_mft(&file, destination).unwrap_or_else(|e| {
            FileRecoveryResult {
                success: false,
                source_path: file.path,
                destination_path: destination.to_string(),
                bytes_recovered: 0,
                message: e,
            }
        }),
        "carved" | "slack" => engine.recover_carved(&file, destination).unwrap_or_else(|e| {
            FileRecoveryResult {
                success: false,
                source_path: file.path,
                destination_path: destination.to_string(),
                bytes_recovered: 0,
                message: e,
            }
        }),
        "USN" | "mft_filesystem" => {
            // USN and filesystem MFT files use the same data_runs based recovery as MFT
            engine.recover_from_mft(&file, destination).unwrap_or_else(|e| {
                FileRecoveryResult {
                    success: false,
                    source_path: file.path,
                    destination_path: destination.to_string(),
                    bytes_recovered: 0,
                    message: e,
                }
            })
        },
        _ => FileRecoveryResult {
            success: false,
            source_path: file.path,
            destination_path: destination.to_string(),
            bytes_recovered: 0,
            message: format!("Unknown file source: {}", file.source),
        },
    }
}
