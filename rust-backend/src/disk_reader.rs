//! Raw Disk Reader Module
//! Provides low-level access to physical drives and partitions for data recovery

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

const SECTOR_SIZE: usize = 512;
const DEFAULT_BUFFER_SIZE: usize = 64 * 1024; // 64KB

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiskInfo {
    pub path: String,
    pub size: u64,
    pub sector_size: u32,
    pub is_removable: bool,
    pub model: String,
    pub serial: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScanProgress {
    pub current_sector: u64,
    pub total_sectors: u64,
    pub percent_complete: f32,
    pub files_found: usize,
    pub bytes_scanned: u64,
    pub status: String,
}

/// Raw disk reader for direct sector access
pub struct DiskReader {
    handle: File,
    sector_size: usize,
    total_size: u64,
    current_position: u64,
}

impl DiskReader {
    /// Open a physical drive or partition for reading
    /// For Windows: Use paths like "\\.\PhysicalDrive0" or "\\.\C:"
    pub fn open(path: &str) -> Result<Self, String> {
        #[cfg(windows)]
        {
            use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE};
            
            // Open with necessary flags for raw disk access
            let file = OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .open(path)
                .map_err(|e| format!("Failed to open disk {}: {}. Run as Administrator.", path, e))?;
            
            // Get disk size
            let size = get_disk_size(&file, path)?;
            
            Ok(DiskReader {
                handle: file,
                sector_size: SECTOR_SIZE,
                total_size: size,
                current_position: 0,
            })
        }
        
        #[cfg(not(windows))]
        {
            let file = OpenOptions::new()
                .read(true)
                .open(path)
                .map_err(|e| format!("Failed to open disk {}: {}", path, e))?;
            
            let metadata = file.metadata().map_err(|e| e.to_string())?;
            
            Ok(DiskReader {
                handle: file,
                sector_size: SECTOR_SIZE,
                total_size: metadata.len(),
                current_position: 0,
            })
        }
    }
    
    /// Open a volume by drive letter (e.g., "C:")
    pub fn open_volume(drive_letter: &str) -> Result<Self, String> {
        let letter = drive_letter.trim_end_matches('\\').trim_end_matches(':');
        let path = format!("\\\\.\\{}:", letter);
        Self::open(&path)
    }
    
    /// Get total disk/volume size
    pub fn size(&self) -> u64 {
        self.total_size
    }
    
    /// Get sector size
    pub fn sector_size(&self) -> usize {
        self.sector_size
    }
    
    /// Get total number of sectors
    pub fn total_sectors(&self) -> u64 {
        self.total_size / self.sector_size as u64
    }
    
    /// Seek to a specific sector
    pub fn seek_sector(&mut self, sector: u64) -> Result<(), String> {
        let byte_offset = sector * self.sector_size as u64;
        self.handle
            .seek(SeekFrom::Start(byte_offset))
            .map_err(|e| format!("Failed to seek to sector {}: {}", sector, e))?;
        self.current_position = byte_offset;
        Ok(())
    }
    
    /// Seek to a specific byte offset
    pub fn seek_bytes(&mut self, offset: u64) -> Result<(), String> {
        self.handle
            .seek(SeekFrom::Start(offset))
            .map_err(|e| format!("Failed to seek to offset {}: {}", offset, e))?;
        self.current_position = offset;
        Ok(())
    }
    
    /// Read a specific number of sectors
    pub fn read_sectors(&mut self, count: usize) -> Result<Vec<u8>, String> {
        let bytes_to_read = count * self.sector_size;
        let mut buffer = vec![0u8; bytes_to_read];
        
        let bytes_read = self.handle
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read sectors: {}", e))?;
        
        self.current_position += bytes_read as u64;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }
    
    /// Read a specific number of bytes
    pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>, String> {
        let mut buffer = vec![0u8; count];
        
        let bytes_read = self.handle
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read bytes: {}", e))?;
        
        self.current_position += bytes_read as u64;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }
    
    /// Read the boot sector (first sector)
    pub fn read_boot_sector(&mut self) -> Result<Vec<u8>, String> {
        self.seek_sector(0)?;
        self.read_sectors(1)
    }
    
    /// Read data at a specific offset without changing position
    pub fn read_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, String> {
        let current = self.current_position;
        self.seek_bytes(offset)?;
        let data = self.read_bytes(size)?;
        self.seek_bytes(current)?;
        Ok(data)
    }
    
    /// Get current position
    pub fn position(&self) -> u64 {
        self.current_position
    }
    
    /// Read MFT from NTFS volume
    pub fn read_mft(&mut self, mft_offset: u64, record_count: usize) -> Result<Vec<u8>, String> {
        self.seek_bytes(mft_offset)?;
        let mft_size = record_count * 1024; // Each MFT record is typically 1024 bytes
        self.read_bytes(mft_size)
    }
    
    /// Scan sectors with a callback for progress
    pub fn scan_with_progress<F>(
        &mut self,
        start_sector: u64,
        sector_count: u64,
        chunk_size: usize,
        mut callback: F,
    ) -> Result<(), String>
    where
        F: FnMut(&[u8], u64, u64) -> bool,
    {
        let sectors_per_chunk = chunk_size / self.sector_size;
        let mut current = start_sector;
        
        while current < start_sector + sector_count {
            let remaining = start_sector + sector_count - current;
            let to_read = std::cmp::min(sectors_per_chunk as u64, remaining) as usize;
            
            self.seek_sector(current)?;
            let data = self.read_sectors(to_read)?;
            
            if !callback(&data, current, current - start_sector) {
                break; // Callback requested stop
            }
            
            current += to_read as u64;
        }
        
        Ok(())
    }
}

/// Get disk size using Windows API
#[cfg(windows)]
fn get_disk_size(file: &File, path: &str) -> Result<u64, String> {
    use std::mem;
    use winapi::um::ioapiset::DeviceIoControl;
    use winapi::um::winioctl::{DISK_GEOMETRY, IOCTL_DISK_GET_DRIVE_GEOMETRY};
    
    unsafe {
        let handle = file.as_raw_handle();
        let mut geometry: DISK_GEOMETRY = mem::zeroed();
        let mut bytes_returned: u32 = 0;
        
        let result = DeviceIoControl(
            handle as *mut _,
            IOCTL_DISK_GET_DRIVE_GEOMETRY,
            std::ptr::null_mut(),
            0,
            &mut geometry as *mut _ as *mut _,
            mem::size_of::<DISK_GEOMETRY>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );
        
        if result != 0 {
            let size = *geometry.Cylinders.QuadPart() as u64
                * geometry.TracksPerCylinder as u64
                * geometry.SectorsPerTrack as u64
                * geometry.BytesPerSector as u64;
            
            if size > 0 {
                return Ok(size);
            }
        }
        
        // Fallback: try seeking to end
        let file_dup = file.try_clone().map_err(|e| e.to_string())?;
        let mut seek_file = file_dup;
        if let Ok(size) = seek_file.seek(SeekFrom::End(0)) {
            if size > 0 {
                return Ok(size);
            }
        }
        
        // Another fallback for volume size
        use winapi::um::winioctl::IOCTL_DISK_GET_LENGTH_INFO;
        
        #[repr(C)]
        struct GET_LENGTH_INFORMATION {
            length: i64,
        }
        
        let mut length_info: GET_LENGTH_INFORMATION = mem::zeroed();
        let result = DeviceIoControl(
            handle as *mut _,
            IOCTL_DISK_GET_LENGTH_INFO,
            std::ptr::null_mut(),
            0,
            &mut length_info as *mut _ as *mut _,
            mem::size_of::<GET_LENGTH_INFORMATION>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );
        
        if result != 0 && length_info.length > 0 {
            return Ok(length_info.length as u64);
        }
        
        Err(format!("Could not determine size of {}", path))
    }
}

/// Save carved data to a file
pub fn save_carved_file(
    data: &[u8],
    destination: &str,
) -> Result<(), String> {
    // Create parent directories if needed
    if let Some(parent) = Path::new(destination).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directories: {}", e))?;
    }
    
    let mut file = File::create(destination)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    
    file.write_all(data)
        .map_err(|e| format!("Failed to write data: {}", e))?;
    
    Ok(())
}

/// Read specific clusters from disk for file recovery
pub fn read_clusters(
    disk: &mut DiskReader,
    cluster_offset: u64,
    cluster_count: u64,
    cluster_size: u32,
) -> Result<Vec<u8>, String> {
    let byte_offset = cluster_offset * cluster_size as u64;
    let byte_count = cluster_count * cluster_size as u64;
    
    disk.seek_bytes(byte_offset)?;
    disk.read_bytes(byte_count as usize)
}

/// Get the physical path for a drive letter
pub fn get_volume_path(drive_letter: &str) -> String {
    let letter = drive_letter
        .trim_end_matches('\\')
        .trim_end_matches(':')
        .to_uppercase();
    format!("\\\\.\\{}:", letter)
}

/// Check if running with required permissions for raw disk access
pub fn check_disk_access_permissions(drive_letter: &str) -> Result<bool, String> {
    let path = get_volume_path(drive_letter);
    
    match DiskReader::open(&path) {
        Ok(_) => Ok(true),
        Err(e) => {
            if e.contains("Administrator") || e.contains("Access") || e.contains("denied") {
                Err("Administrator privileges required for raw disk access".to_string())
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_volume_path() {
        assert_eq!(get_volume_path("C:"), "\\\\.\\C:");
        assert_eq!(get_volume_path("C:\\"), "\\\\.\\C:");
        assert_eq!(get_volume_path("D"), "\\\\.\\D:");
    }
}
