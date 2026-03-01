//! File System Disk Reader Module
//! Provides access to encrypted drives through Windows file system APIs
//! Uses low-level Windows APIs with backup semantics to access protected files

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::os::windows::io::FromRawHandle;

const SECTOR_SIZE: usize = 512;
const MFT_RECORD_SIZE: u64 = 1024;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSystemDiskInfo {
    pub drive_letter: String,
    pub size: u64,
    pub is_encrypted: bool,
}

/// Represents a deleted file found in the USN Change Journal
#[derive(Debug, Clone)]
pub struct UsnDeletedFile {
    pub file_name: String,
    pub mft_record: u64,
    pub parent_mft_record: u64,
    pub timestamp: i64,        // Unix timestamp of deletion
    pub file_attributes: u32,
    pub reason: u32,
}

/// Physical extent of $MFT on disk (for fragmentation-aware reading)
/// Extents are stored in logical order: extent 0 is first logically, etc.
struct MftExtent {
    physical_cluster: u64,
    cluster_count: u64,
}

/// File system-based disk reader for encrypted drives
/// Uses Windows CreateFile with backup semantics to access volume
pub struct FileSystemDiskReader {
    drive_letter: String,
    sector_size: usize,
    mft_handle: Option<File>,       // Volume handle for raw access
    mft_file_handle: Option<File>,  // $MFT file handle (fragmentation-safe)
    volume_handle: Option<File>,
    mft_offset: u64,  // Byte offset of MFT from volume start
    mft_record_size: u64,  // Actual MFT record size from boot sector (usually 1024, can be 4096)
    cluster_size: u64,  // Actual NTFS cluster size from boot sector (usually 4096)
    mft_file_open_attempted: bool,  // Track whether we already tried opening $MFT file
    mft_extents: Vec<MftExtent>,    // $MFT data run extents for direct MFT reading
    mft_extents_built: bool,        // Whether we've built the extent map
}

#[cfg(windows)]
mod win_api {
    use winapi::um::fileapi::{CreateFileW, OPEN_EXISTING};
    use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
    use winapi::um::winnt::{
        FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_SHARE_DELETE,
        GENERIC_READ, FILE_ATTRIBUTE_NORMAL, HANDLE,
    };
    use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::AdjustTokenPrivileges;
    use winapi::um::winnt::{
        TOKEN_ADJUST_PRIVILEGES, TOKEN_QUERY, SE_PRIVILEGE_ENABLED,
        TOKEN_PRIVILEGES, LUID,
    };
    use winapi::um::winbase::LookupPrivilegeValueW;
    use std::ptr::null_mut;

    /// Enable backup privilege to access protected system files
    pub fn enable_backup_privilege() -> Result<(), String> {
        unsafe {
            let mut token_handle: HANDLE = null_mut();
            
            // Open process token
            if OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token_handle
            ) == 0 {
                return Err(format!("Failed to open process token: error {}", GetLastError()));
            }
            
            // Look up the backup privilege LUID
            let privilege_name: Vec<u16> = "SeBackupPrivilege\0".encode_utf16().collect();
            let mut luid: LUID = std::mem::zeroed();
            
            if LookupPrivilegeValueW(null_mut(), privilege_name.as_ptr(), &mut luid) == 0 {
                CloseHandle(token_handle);
                return Err(format!("Failed to lookup privilege: error {}", GetLastError()));
            }
            
            // Set up TOKEN_PRIVILEGES structure
            let mut tp: TOKEN_PRIVILEGES = std::mem::zeroed();
            tp.PrivilegeCount = 1;
            tp.Privileges[0].Luid = luid;
            tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
            
            // Adjust token privileges
            if AdjustTokenPrivileges(
                token_handle,
                0,
                &mut tp,
                std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
                null_mut(),
                null_mut()
            ) == 0 {
                CloseHandle(token_handle);
                return Err(format!("Failed to adjust privileges: error {}", GetLastError()));
            }
            
            let err = GetLastError();
            CloseHandle(token_handle);
            
            // ERROR_NOT_ALL_ASSIGNED = 1300
            if err == 1300 {
                return Err("SeBackupPrivilege not available. Run as Administrator.".to_string());
            }
            
            Ok(())
        }
    }

    /// Open a file with backup semantics (can access system files like $MFT)
    pub fn open_with_backup_semantics(path: &str) -> Result<HANDLE, String> {
        let wide_path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        
        unsafe {
            let handle = CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_ATTRIBUTE_NORMAL,
                null_mut()
            );
            
            if handle == INVALID_HANDLE_VALUE {
                let error = GetLastError();
                return Err(format!("CreateFileW failed for '{}': error code {}", path, error));
            }
            
            Ok(handle)
        }
    }
}

impl FileSystemDiskReader {
    /// Create a new file system disk reader for a drive letter
    pub fn new(drive_letter: &str) -> Result<Self, String> {
        let letter = drive_letter.trim_end_matches('\\').trim_end_matches(':');
        
        Ok(FileSystemDiskReader {
            drive_letter: letter.to_string(),
            sector_size: SECTOR_SIZE,
            mft_handle: None,
            mft_file_handle: None,
            volume_handle: None,
            mft_offset: 0,
            mft_record_size: MFT_RECORD_SIZE,  // Default, will be updated from boot sector
            cluster_size: 4096,  // Default, will be updated from boot sector
            mft_file_open_attempted: false,
            mft_extents: Vec::new(),
            mft_extents_built: false,
        })
    }
    
    /// Enable backup privilege - required to access protected files
    #[cfg(windows)]
    pub fn enable_privileges() -> Result<(), String> {
        win_api::enable_backup_privilege()
    }
    
    #[cfg(not(windows))]
    pub fn enable_privileges() -> Result<(), String> {
        Err("Only supported on Windows".to_string())
    }
    
    /// Open the volume for reading MFT
    /// For BitLocker encrypted drives, opening \\.\C: gives decrypted data
    /// when the volume is unlocked
    pub fn open_mft(&mut self) -> Result<(), String> {
        #[cfg(windows)]
        {
            // First ensure we have backup privilege
            win_api::enable_backup_privilege()?;
            
            // Open the volume - Windows will give us decrypted data
            let volume_path = format!(r"\\.\{}:", self.drive_letter);
            
            let handle = win_api::open_with_backup_semantics(&volume_path)?;
            
            // Convert raw handle to Rust File
            let file = unsafe { File::from_raw_handle(handle as *mut std::ffi::c_void) };
            
            self.mft_handle = Some(file);
            
            // Read and store MFT offset from boot sector
            let mft_offset = self.read_mft_location()?;
            self.mft_offset = mft_offset;
            
            Ok(())
        }
        
        #[cfg(not(windows))]
        {
            Err("File system mode only supported on Windows".to_string())
        }
    }
    
    /// Read MFT location from NTFS boot sector
    #[cfg(windows)]
    fn read_mft_location(&mut self) -> Result<u64, String> {
        use std::io::{Read, Seek, SeekFrom};
        
        let handle = self.mft_handle.as_mut()
            .ok_or("Volume not opened")?;
        
        // Seek to beginning
        handle.seek(SeekFrom::Start(0))
            .map_err(|e| format!("Failed to seek to boot sector: {}", e))?;
        
        // Read boot sector (first 512 bytes)
        let mut boot_sector = vec![0u8; 512];
        handle.read_exact(&mut boot_sector)
            .map_err(|e| format!("Failed to read boot sector: {}", e))?;
        
        // Check NTFS signature
        if &boot_sector[3..7] != b"NTFS" {
            return Err("Not an NTFS volume".to_string());
        }
        
        // Get bytes per sector and sectors per cluster
        let bytes_per_sector = u16::from_le_bytes([boot_sector[11], boot_sector[12]]) as u64;
        let sectors_per_cluster = boot_sector[13] as u64;
        let cluster_size = bytes_per_sector * sectors_per_cluster;
        
        // Get actual MFT record size from boot sector offset 0x40
        let mft_size_raw = boot_sector[0x40] as i8;
        let actual_record_size = if mft_size_raw > 0 {
            (mft_size_raw as u64) * cluster_size
        } else {
            1u64 << ((-mft_size_raw) as u64)
        };
        self.mft_record_size = actual_record_size;
        self.cluster_size = cluster_size;
        eprintln!("[MFT] Boot sector: bytes_per_sector={}, sectors_per_cluster={}, cluster_size={}", 
            bytes_per_sector, sectors_per_cluster, cluster_size);
        eprintln!("[MFT] MFT record size from boot sector: {} bytes (raw value: {})", 
            actual_record_size, mft_size_raw);
        
        // Get MFT cluster number (offset 0x30, 8 bytes)
        let mft_cluster = u64::from_le_bytes([
            boot_sector[0x30], boot_sector[0x31], boot_sector[0x32], boot_sector[0x33],
            boot_sector[0x34], boot_sector[0x35], boot_sector[0x36], boot_sector[0x37],
        ]);
        
        // Calculate MFT byte offset
        let mft_offset = mft_cluster * cluster_size;
        
        Ok(mft_offset)
    }
    
    #[cfg(not(windows))]
    fn read_mft_location(&mut self) -> Result<u64, String> {
        Err("Only supported on Windows".to_string())
    }
    
    /// Open the $MFT file directly through Windows filesystem API
    /// This is the BEST way to read MFT - Windows handles:
    /// - MFT fragmentation (data runs)
    /// - BitLocker decryption
    /// - Record N is at simple offset N * record_size
    /// - ALL records readable including freed/deleted slots
    #[cfg(windows)]
    fn open_mft_file(&mut self) -> Result<(), String> {
        self.mft_file_open_attempted = true;
        
        // Try to open $MFT directly - works on some Windows versions with admin + backup semantics
        let mft_path = format!("{}:\\$MFT", self.drive_letter);
        match win_api::open_with_backup_semantics(&mft_path) {
            Ok(handle) => {
                let file = unsafe { File::from_raw_handle(handle as *mut std::ffi::c_void) };
                self.mft_file_handle = Some(file);
                eprintln!("[MFT]: Successfully opened $MFT file directly — best method for reading all records");
                Ok(())
            }
            Err(e) => {
                eprintln!("[MFT]: Cannot open $MFT directly ({}), will use FSCTL + data-run mapping", e);
                Err(e)
            }
        }
    }
    
    /// Read a single MFT record using FSCTL_GET_NTFS_FILE_RECORD
    /// This is the most reliable way - Windows handles fragmentation + BitLocker decryption
    #[cfg(windows)]
    fn read_mft_record_via_ioctl(&mut self, record_number: u64) -> Result<Vec<u8>, String> {
        use winapi::um::ioapiset::DeviceIoControl;
        use std::os::windows::io::AsRawHandle;
        
        // Open volume if not already open
        if self.volume_handle.is_none() {
            self.open_volume()?;
        }
        
        let volume = self.volume_handle.as_ref().ok_or("No volume handle")?;
        let handle = volume.as_raw_handle() as winapi::um::winnt::HANDLE;
        
        // FSCTL_GET_NTFS_FILE_RECORD = 0x00090068
        const FSCTL_GET_NTFS_FILE_RECORD: u32 = 0x00090068;
        
        // Input: NTFS_FILE_RECORD_INPUT_BUFFER - just the file reference number (i64)
        #[repr(C)]
        struct NtfsFileRecordInput {
            file_reference_number: i64,
        }
        
        let input = NtfsFileRecordInput {
            file_reference_number: record_number as i64,
        };
        
        // Output buffer: NTFS_FILE_RECORD_OUTPUT_BUFFER
        // Header: FileReferenceNumber (8 bytes) + FileRecordLength (4 bytes) = 12 bytes header
        // Then: FileRecordBuffer (variable length)
        let header_size = 12usize; // 8 (FileReferenceNumber) + 4 (FileRecordLength)
        let output_size = header_size + self.mft_record_size as usize + 256; // Extra padding
        let mut output_buffer = vec![0u8; output_size];
        let mut bytes_returned: u32 = 0;
        
        let result = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_GET_NTFS_FILE_RECORD,
                &input as *const _ as *mut std::ffi::c_void,
                std::mem::size_of::<NtfsFileRecordInput>() as u32,
                output_buffer.as_mut_ptr() as *mut std::ffi::c_void,
                output_size as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };
        
        if result == 0 {
            let err = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(format!("FSCTL_GET_NTFS_FILE_RECORD failed for record {}: error {}", record_number, err));
        }
        
        // Read the actual record length from the output buffer
        let record_length = if bytes_returned >= 12 {
            u32::from_le_bytes([
                output_buffer[8], output_buffer[9], output_buffer[10], output_buffer[11]
            ]) as usize
        } else {
            return Err(format!("FSCTL returned only {} bytes", bytes_returned));
        };
        
        if bytes_returned < (header_size + record_length) as u32 {
            return Err(format!("FSCTL returned {} bytes, need at least {}", 
                bytes_returned, header_size + record_length));
        }
        
        // CRITICAL: Verify the returned FileReferenceNumber matches what we requested.
        // FSCTL_GET_NTFS_FILE_RECORD returns the nearest IN-USE record when the
        // requested one is freed (deleted).  If the returned record differs from
        // what we asked for, the slot is freed → treat as error so we don't
        // parse a different file's data under the wrong record number.
        if bytes_returned >= 8 {
            let returned_ref = u64::from_le_bytes([
                output_buffer[0], output_buffer[1], output_buffer[2], output_buffer[3],
                output_buffer[4], output_buffer[5], output_buffer[6], output_buffer[7],
            ]);
            // Lower 48 bits = record number, upper 16 bits = sequence number
            let returned_record_num = returned_ref & 0x0000_FFFF_FFFF_FFFF;
            if returned_record_num != record_number {
                return Err(format!(
                    "FSCTL returned record {} instead of requested {} (slot freed/deleted)",
                    returned_record_num, record_number
                ));
            }
        }
        
        // The actual MFT record data starts at offset 12 (after FileReferenceNumber + FileRecordLength)
        let end = header_size + record_length.min(self.mft_record_size as usize);
        let record_data = output_buffer[header_size..end].to_vec();
        
        Ok(record_data)
    }
    
    /// Build a map of $MFT's physical extents by reading MFT record 0's DATA attribute.
    /// This enables reading ANY MFT record (including freed/deleted slots) by computing
    /// its physical disk location from the MFT's own data runs.
    #[cfg(windows)]
    fn build_mft_data_run_map(&mut self) -> Result<(), String> {
        if self.mft_extents_built {
            return Ok(());
        }
        self.mft_extents_built = true;
        
        // Read MFT record 0 via FSCTL — record 0 ($MFT itself) is always in-use
        let record0 = self.read_mft_record_via_ioctl(0)?;
        
        if record0.len() < 56 || &record0[0..4] != b"FILE" {
            return Err("MFT record 0 invalid".to_string());
        }
        
        // Walk attributes to find the unnamed DATA (0x80) attribute
        let first_attr = u16::from_le_bytes([record0[0x14], record0[0x15]]) as usize;
        let mut offset = first_attr;
        
        while offset + 8 < record0.len() {
            let attr_type = u32::from_le_bytes([
                record0[offset], record0[offset+1], record0[offset+2], record0[offset+3]
            ]);
            if attr_type == 0xFFFFFFFF || attr_type == 0 { break; }
            
            let attr_len = u32::from_le_bytes([
                record0[offset+4], record0[offset+5], record0[offset+6], record0[offset+7]
            ]) as usize;
            if attr_len == 0 || offset + attr_len > record0.len() { break; }
            
            // DATA attribute = 0x80, non-resident (byte at offset+8 != 0)
            if attr_type == 0x80 && record0[offset + 8] != 0 {
                // Check that this is the unnamed stream (name_length = 0 at offset+9)
                let name_length = record0[offset + 9];
                if name_length != 0 {
                    offset += attr_len;
                    continue;
                }
                
                // Data runs start at the offset given at attribute+32
                let runs_offset = u16::from_le_bytes([
                    record0[offset + 32], record0[offset + 33]
                ]) as usize;
                
                if offset + runs_offset >= record0.len() { break; }
                let runs_end = (offset + attr_len).min(record0.len());
                let runs_data = &record0[offset + runs_offset..runs_end];
                
                // Parse data runs using the shared NTFS parser
                let data_runs = crate::ntfs_parser::parse_data_runs(runs_data);
                
                self.mft_extents.clear();
                for run in &data_runs {
                    if run.cluster_offset > 0 && run.cluster_count > 0 {
                        self.mft_extents.push(MftExtent {
                            physical_cluster: run.cluster_offset as u64,
                            cluster_count: run.cluster_count,
                        });
                    }
                }
                
                let total_clusters: u64 = self.mft_extents.iter().map(|e| e.cluster_count).sum();
                let total_bytes = total_clusters * self.cluster_size;
                let approx_records = total_bytes / self.mft_record_size;
                eprintln!("[MFT-MAP]: Built extent map: {} extents, {} clusters, ~{} records",
                    self.mft_extents.len(), total_clusters, approx_records);
                
                return Ok(());
            }
            
            offset += attr_len;
        }
        
        Err("Could not find DATA attribute in MFT record 0".to_string())
    }
    
    /// Read an MFT record by computing its physical location from the MFT extent map.
    /// This works for ALL records including freed/deleted ones — bypasses FSCTL limitations.
    #[cfg(windows)]
    fn read_mft_record_via_data_runs(&mut self, record_number: u64) -> Result<Vec<u8>, String> {
        // Ensure the extent map is built
        if !self.mft_extents_built {
            self.build_mft_data_run_map()?;
        }
        
        if self.mft_extents.is_empty() {
            return Err("No MFT extents available".to_string());
        }
        
        let record_size = self.mft_record_size;
        let cluster_size = self.cluster_size;
        
        // Calculate which logical MFT cluster this record lives in
        let logical_byte = record_number * record_size;
        let logical_cluster = logical_byte / cluster_size;
        let offset_in_cluster = (logical_byte % cluster_size) as usize;
        
        // Walk through extents to find the matching physical cluster
        let mut logical_start = 0u64;
        for extent in &self.mft_extents {
            let logical_end = logical_start + extent.cluster_count;
            if logical_cluster >= logical_start && logical_cluster < logical_end {
                let cluster_in_extent = logical_cluster - logical_start;
                let physical_cluster = extent.physical_cluster + cluster_in_extent;
                let physical_byte = physical_cluster * cluster_size + offset_in_cluster as u64;
                
                // Read from volume handle
                if self.volume_handle.is_none() {
                    self.open_volume()?;
                }
                let file = self.volume_handle.as_mut().ok_or("No volume handle")?;
                file.seek(SeekFrom::Start(physical_byte))
                    .map_err(|e| format!("Seek to MFT record {} failed: {}", record_number, e))?;
                
                let mut buffer = vec![0u8; record_size as usize];
                file.read_exact(&mut buffer)
                    .map_err(|e| format!("Read MFT record {} failed: {}", record_number, e))?;
                
                return Ok(buffer);
            }
            logical_start = logical_end;
        }
        
        Err(format!("MFT record {} beyond extent map (logical cluster {})", record_number, logical_cluster))
    }
    
    /// Read MFT records - Windows handles decryption automatically
    ///
    /// Fallback chain:
    /// 1. $MFT file handle — best method, reads ALL records including freed/deleted
    /// 2. FSCTL_GET_NTFS_FILE_RECORD — reliable for in-use records only
    /// 3. MFT data-run mapping — reads physical bytes, works for freed records
    /// 4. Raw volume offset — last resort, only works if MFT is not fragmented
    pub fn read_mft_record(&mut self, record_number: u64) -> Result<Vec<u8>, String> {
        let record_size = self.mft_record_size;
        
        // Method 1: Use $MFT file handle if available (try opening once)
        if self.mft_file_handle.is_none() && !self.mft_file_open_attempted {
            let _ = self.open_mft_file();
        }
        if let Some(handle) = self.mft_file_handle.as_mut() {
            let offset = record_number * record_size;
            if let Ok(_) = handle.seek(SeekFrom::Start(offset)) {
                let mut buffer = vec![0u8; record_size as usize];
                if let Ok(_) = handle.read_exact(&mut buffer) {
                    return Ok(buffer);
                }
            }
        }
        
        // Method 2: FSCTL_GET_NTFS_FILE_RECORD — works for in-use records
        // For freed records, FSCTL returns a different record number; we detect this
        // and fall through to Method 3 which can read freed slots.
        #[cfg(windows)]
        {
            match self.read_mft_record_via_ioctl(record_number) {
                Ok(buffer) => return Ok(buffer),
                Err(_) => {
                    // Freed slot or other FSCTL error — try data-run mapping next
                }
            }
        }
        
        // Method 3: MFT data-run mapping — reads actual physical bytes on disk.
        // This handles MFT fragmentation AND can read freed/deleted record slots
        // that FSCTL refuses to return. This is the key method for finding deleted files.
        #[cfg(windows)]
        {
            match self.read_mft_record_via_data_runs(record_number) {
                Ok(buffer) => return Ok(buffer),
                Err(_) => {
                    // Data-run map not available or record beyond extents
                }
            }
        }
        
        // Method 4: Fallback to raw volume offset (works for non-fragmented MFT)
        if self.mft_handle.is_none() {
            self.open_mft()?;
        }
        
        let offset = self.mft_offset + (record_number * record_size);
        let handle = self.mft_handle.as_mut().unwrap();
        
        handle
            .seek(SeekFrom::Start(offset))
            .map_err(|e| format!("Failed to seek to MFT record {}: {}", record_number, e))?;
        
        let mut buffer = vec![0u8; record_size as usize];
        handle
            .read_exact(&mut buffer)
            .map_err(|e| format!("Failed to read MFT record {}: {}", record_number, e))?;
        
        Ok(buffer)
    }
    
    /// Read multiple MFT records at once
    pub fn read_mft_records(&mut self, start_record: u64, count: usize) -> Result<Vec<Vec<u8>>, String> {
        let mut records = Vec::with_capacity(count);
        
        for i in 0..count {
            match self.read_mft_record(start_record + i as u64) {
                Ok(record) => records.push(record),
                Err(_e) => {
                    // Stop on first error (might be end of MFT)
                    break;
                }
            }
        }
        
        Ok(records)
    }
    
    /// Read file clusters through volume handle
    /// Uses backup semantics for proper access
    pub fn read_clusters(&mut self, cluster_offset: u64, cluster_count: u64, cluster_size: u64) -> Result<Vec<u8>, String> {
        // Calculate byte offset
        let byte_offset = cluster_offset * cluster_size;
        let byte_size = cluster_count * cluster_size;
        
        #[cfg(windows)]
        {
            // Open volume if not already open
            if self.volume_handle.is_none() {
                let volume_path = format!(r"\\.\{}:", self.drive_letter);
                let handle = win_api::open_with_backup_semantics(&volume_path)?;
                let file = unsafe { File::from_raw_handle(handle as *mut std::ffi::c_void) };
                self.volume_handle = Some(file);
            }
            
            let file = self.volume_handle.as_mut().unwrap();
            
            // Seek to the cluster position
            file.seek(SeekFrom::Start(byte_offset))
                .map_err(|e| format!("Failed to seek to cluster offset {}: {}", byte_offset, e))?;
            
            // Read the data
            let mut buffer = vec![0u8; byte_size as usize];
            file.read_exact(&mut buffer)
                .map_err(|e| format!("Failed to read {} bytes: {}", byte_size, e))?;
            
            Ok(buffer)
        }
        
        #[cfg(not(windows))]
        {
            let _ = (byte_offset, byte_size);
            Err("File system mode only supported on Windows".to_string())
        }
    }
    
    /// Open the volume handle for direct cluster reading
    #[cfg(windows)]
    pub fn open_volume(&mut self) -> Result<(), String> {
        if self.volume_handle.is_none() {
            win_api::enable_backup_privilege()?;
            let volume_path = format!(r"\\.\{}:", self.drive_letter);
            let handle = win_api::open_with_backup_semantics(&volume_path)?;
            let file = unsafe { File::from_raw_handle(handle as *mut std::ffi::c_void) };
            self.volume_handle = Some(file);
        }
        Ok(())
    }
    
    #[cfg(not(windows))]
    pub fn open_volume(&mut self) -> Result<(), String> {
        Err("Only supported on Windows".to_string())
    }
    
    /// Save recovered file data
    pub fn save_file(&self, data: &[u8], output_path: &str) -> Result<(), String> {
        use std::io::Write;
        
        let mut file = File::create(output_path)
            .map_err(|e| format!("Failed to create output file {}: {}", output_path, e))?;
        
        file.write_all(data)
            .map_err(|e| format!("Failed to write file data: {}", e))?;
        
        Ok(())
    }
    
    /// Get drive letter
    pub fn drive_letter(&self) -> &str {
        &self.drive_letter
    }
    
    /// Get actual NTFS cluster size (read from boot sector during open_mft)
    /// Returns correct value only after test_access() or open_mft() has been called.
    pub fn get_cluster_size(&self) -> u64 {
        self.cluster_size
    }
    
    /// Get sector size
    pub fn sector_size(&self) -> usize {
        self.sector_size
    }
    
    /// Test if we can access the drive
    pub fn test_access(&mut self) -> Result<(), String> {
        // Try to open and read first MFT record
        self.open_mft()?;
        self.read_mft_record(0)?;
        Ok(())
    }
    
    /// Get MFT file total size by reading $MFT record 0 data attribute
    #[cfg(windows)]
    pub fn get_mft_total_records(&mut self) -> Result<u64, String> {
        let buffer = self.read_mft_record(0)?;
        
        // Parse $MFT record to find the DATA attribute and get total MFT size
        if buffer.len() < 42 {
            return Err("MFT record 0 too small".to_string());
        }
        
        // First attribute offset at 0x14
        let first_attr = u16::from_le_bytes([buffer[0x14], buffer[0x15]]) as usize;
        let mut offset = first_attr;
        
        while offset + 4 < buffer.len() {
            let attr_type = u32::from_le_bytes([
                buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]
            ]);
            
            if attr_type == 0xFFFFFFFF || attr_type == 0 { break; }
            
            let attr_len = u32::from_le_bytes([
                buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]
            ]) as usize;
            
            if attr_len == 0 || offset + attr_len > buffer.len() { break; }
            
            // DATA attribute = 0x80
            if attr_type == 0x80 {
                // Check if non-resident (byte at offset+8)
                if offset + 0x38 < buffer.len() && buffer[offset + 8] != 0 {
                    // Non-resident: real size at offset 0x30 within attribute
                    let real_size = u64::from_le_bytes([
                        buffer[offset+0x30], buffer[offset+0x31], buffer[offset+0x32], buffer[offset+0x33],
                        buffer[offset+0x34], buffer[offset+0x35], buffer[offset+0x36], buffer[offset+0x37],
                    ]);
                    let total_records = real_size / self.mft_record_size;
                    eprintln!("DEBUG [FS]: $MFT total size: {} bytes = {} records (record_size={})", 
                        real_size, total_records, self.mft_record_size);
                    return Ok(total_records);
                }
            }
            
            offset += attr_len;
        }
        
        Err("Could not find $MFT DATA attribute".to_string())
    }
    
    #[cfg(not(windows))]
    pub fn get_mft_total_records(&mut self) -> Result<u64, String> {
        Err("Only supported on Windows".to_string())
    }
    
    /// Scan USN Change Journal for recently deleted files
    /// This works on BitLocker drives through the volume handle
    /// Returns: Vec<(file_name, parent_frn, file_ref_number, timestamp, reason)>
    #[cfg(windows)]
    pub fn scan_usn_journal(&mut self) -> Result<Vec<UsnDeletedFile>, String> {
        use winapi::um::ioapiset::DeviceIoControl;
        use std::os::windows::io::AsRawHandle;
        
        // Open volume if needed
        if self.volume_handle.is_none() {
            self.open_volume()?;
        }
        
        let volume = self.volume_handle.as_ref().unwrap();
        let handle = volume.as_raw_handle() as winapi::um::winnt::HANDLE;
        
        // FSCTL constants
        const FSCTL_QUERY_USN_JOURNAL: u32 = 0x000900f4;
        const FSCTL_READ_USN_JOURNAL: u32 = 0x000900bb;
        
        // Step 1: Query USN journal info
        #[repr(C)]
        #[derive(Default)]
        struct USN_JOURNAL_DATA {
            usn_journal_id: u64,
            first_usn: i64,
            next_usn: i64,
            lowest_valid_usn: i64,
            max_usn: i64,
            maximum_size: u64,
            allocation_delta: u64,
        }
        
        let mut journal_data = USN_JOURNAL_DATA::default();
        let mut bytes_returned: u32 = 0;
        
        let result = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_QUERY_USN_JOURNAL,
                std::ptr::null_mut(),
                0,
                &mut journal_data as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<USN_JOURNAL_DATA>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };
        
        if result == 0 {
            let err = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(format!("FSCTL_QUERY_USN_JOURNAL failed: error {}", err));
        }
        
        eprintln!("DEBUG [USN]: Journal ID: {}, First USN: {}, Next USN: {}", 
            journal_data.usn_journal_id, journal_data.first_usn, journal_data.next_usn);
        
        // Step 2: Read USN records looking for deletions
        #[repr(C)]
        struct READ_USN_JOURNAL_DATA {
            start_usn: i64,
            reason_mask: u32,
            return_only_on_close: u32,
            timeout: u64,
            bytes_to_wait_for: u64,
            usn_journal_id: u64,
        }
        
        // USN_REASON_FILE_DELETE = 0x200, USN_REASON_CLOSE = 0x80000000
        const USN_REASON_FILE_DELETE: u32 = 0x00000200;
        
        let mut read_data = READ_USN_JOURNAL_DATA {
            start_usn: journal_data.first_usn,
            reason_mask: USN_REASON_FILE_DELETE,  // Only deletion events
            return_only_on_close: 0,
            timeout: 0,
            bytes_to_wait_for: 0,
            usn_journal_id: journal_data.usn_journal_id,
        };
        
        let buffer_size = 65536usize;
        let mut buffer = vec![0u8; buffer_size];
        let mut deleted_files: Vec<UsnDeletedFile> = Vec::new();
        let mut total_records_read = 0u64;
        
        loop {
            let mut bytes_returned: u32 = 0;
            
            let result = unsafe {
                DeviceIoControl(
                    handle,
                    FSCTL_READ_USN_JOURNAL,
                    &mut read_data as *mut _ as *mut std::ffi::c_void,
                    std::mem::size_of::<READ_USN_JOURNAL_DATA>() as u32,
                    buffer.as_mut_ptr() as *mut std::ffi::c_void,
                    buffer_size as u32,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                )
            };
            
            if result == 0 {
                let err = unsafe { winapi::um::errhandlingapi::GetLastError() };
                // ERROR_HANDLE_EOF (38) or ERROR_WRITE_PROTECT (19) means we've read everything
                if err == 38 || err == 1181 {
                    break;
                }
                eprintln!("DEBUG [USN]: Read failed with error {}, stopping", err);
                break;
            }
            
            if bytes_returned < 8 {
                break;
            }
            
            // First 8 bytes is the next USN to read
            let next_usn = i64::from_le_bytes([
                buffer[0], buffer[1], buffer[2], buffer[3],
                buffer[4], buffer[5], buffer[6], buffer[7],
            ]);
            
            // Parse USN_RECORD_V2 structures after the first 8 bytes
            let mut offset = 8usize;
            
            while offset + 64 < bytes_returned as usize {
                // USN_RECORD_V2 structure
                let record_length = u32::from_le_bytes([
                    buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]
                ]) as usize;
                
                if record_length == 0 || offset + record_length > bytes_returned as usize {
                    break;
                }
                
                // Parse fields
                let file_ref = u64::from_le_bytes([
                    buffer[offset+8], buffer[offset+9], buffer[offset+10], buffer[offset+11],
                    buffer[offset+12], buffer[offset+13], buffer[offset+14], buffer[offset+15],
                ]);
                
                let parent_ref = u64::from_le_bytes([
                    buffer[offset+16], buffer[offset+17], buffer[offset+18], buffer[offset+19],
                    buffer[offset+20], buffer[offset+21], buffer[offset+22], buffer[offset+23],
                ]);
                
                let timestamp = i64::from_le_bytes([
                    buffer[offset+32], buffer[offset+33], buffer[offset+34], buffer[offset+35],
                    buffer[offset+36], buffer[offset+37], buffer[offset+38], buffer[offset+39],
                ]);
                
                let reason = u32::from_le_bytes([
                    buffer[offset+40], buffer[offset+41], buffer[offset+42], buffer[offset+43]
                ]);
                
                let file_attributes = u32::from_le_bytes([
                    buffer[offset+52], buffer[offset+53], buffer[offset+54], buffer[offset+55]
                ]);
                
                let file_name_length = u16::from_le_bytes([
                    buffer[offset+56], buffer[offset+57]
                ]) as usize;
                
                let file_name_offset = u16::from_le_bytes([
                    buffer[offset+58], buffer[offset+59]
                ]) as usize;
                
                // Extract file name (UTF-16LE)
                let name_start = offset + file_name_offset;
                let name_end = name_start + file_name_length;
                
                if name_end <= bytes_returned as usize && file_name_length > 0 {
                    let name_bytes: Vec<u16> = (0..file_name_length/2)
                        .map(|i| u16::from_le_bytes([
                            buffer[name_start + i*2], buffer[name_start + i*2 + 1]
                        ]))
                        .collect();
                    
                    let file_name = String::from_utf16_lossy(&name_bytes);
                    
                    // Only include deletion events for files (not directories)
                    let is_directory = (file_attributes & 0x10) != 0;
                    
                    if (reason & USN_REASON_FILE_DELETE) != 0 && !is_directory {
                        // Extract MFT record number (lower 48 bits of file reference)
                        let mft_record = file_ref & 0x0000FFFFFFFFFFFF;
                        let parent_mft_record = parent_ref & 0x0000FFFFFFFFFFFF;
                        
                        // Convert Windows FILETIME to unix timestamp
                        let unix_time = if timestamp > 0 {
                            (timestamp - 116444736000000000) / 10000000
                        } else {
                            0
                        };
                        
                        deleted_files.push(UsnDeletedFile {
                            file_name,
                            mft_record,
                            parent_mft_record,
                            timestamp: unix_time,
                            file_attributes,
                            reason,
                        });
                    }
                }
                
                total_records_read += 1;
                offset += record_length;
            }
            
            // Update start USN for next batch
            if next_usn <= read_data.start_usn {
                break;
            }
            read_data.start_usn = next_usn;
            
            // Safety: limit total records
            if total_records_read > 10_000_000 {
                eprintln!("DEBUG [USN]: Hit 10M record limit, stopping");
                break;
            }
        }
        
        eprintln!("DEBUG [USN]: Scanned {} USN records, found {} deleted files", 
            total_records_read, deleted_files.len());
        
        Ok(deleted_files)
    }
    
    #[cfg(not(windows))]
    pub fn scan_usn_journal(&mut self) -> Result<Vec<UsnDeletedFile>, String> {
        Ok(Vec::new())
    }
}

/// Helper function to check if a drive is accessible through file system API
/// Attempts to open $MFT with backup semantics
#[cfg(windows)]
pub fn check_filesystem_access(drive_letter: &str) -> Result<bool, String> {
    let letter = drive_letter.trim_end_matches('\\').trim_end_matches(':');
    let mft_path = format!(r"\\.\{}:\$MFT", letter);
    
    // Try to enable backup privilege first
    if win_api::enable_backup_privilege().is_err() {
        return Ok(false);
    }
    
    match win_api::open_with_backup_semantics(&mft_path) {
        Ok(handle) => {
            // Close the handle
            unsafe {
                winapi::um::handleapi::CloseHandle(handle);
            }
            Ok(true)
        },
        Err(_) => Ok(false),
    }
}

#[cfg(not(windows))]
pub fn check_filesystem_access(_drive_letter: &str) -> Result<bool, String> {
    Ok(false)
}
