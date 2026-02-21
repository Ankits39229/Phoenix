//! NTFS MFT Parser Module
//! Parses the Master File Table to find deleted file records

use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Seek, SeekFrom};

// NTFS Constants
const MFT_RECORD_SIZE: usize = 1024;
const MFT_SIGNATURE: &[u8] = b"FILE";
const ATTRIBUTE_END: u32 = 0xFFFFFFFF;

// Attribute Types
const ATTRIBUTE_STANDARD_INFORMATION: u32 = 0x10;
const ATTRIBUTE_FILE_NAME: u32 = 0x30;
const ATTRIBUTE_DATA: u32 = 0x80;

// File Attribute Flags
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10000000;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MftEntry {
    pub record_number: u64,
    pub file_name: String,
    pub parent_record: u64,
    pub file_size: u64,
    pub allocated_size: u64,
    pub created_time: i64,
    pub modified_time: i64,
    pub accessed_time: i64,
    pub is_deleted: bool,
    pub is_directory: bool,
    pub is_in_use: bool,
    pub data_runs: Vec<DataRun>,
    pub extension: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataRun {
    pub cluster_offset: i64,
    pub cluster_count: u64,
}

#[derive(Debug)]
pub struct NtfsBootSector {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub mft_cluster: u64,
    pub mft_record_size: u32,
    pub cluster_size: u32,
}

/// Parse NTFS boot sector to get MFT location
pub fn parse_boot_sector(data: &[u8]) -> Option<NtfsBootSector> {
    if data.len() < 512 {
        return None;
    }
    
    // Check for NTFS signature
    if &data[3..7] != b"NTFS" {
        return None;
    }
    
    let mut cursor = Cursor::new(data);
    
    // Bytes per sector at offset 0x0B
    cursor.seek(SeekFrom::Start(0x0B)).ok()?;
    let bytes_per_sector = cursor.read_u16::<LittleEndian>().ok()?;
    
    // Sectors per cluster at offset 0x0D
    cursor.seek(SeekFrom::Start(0x0D)).ok()?;
    let sectors_per_cluster = cursor.read_u8().ok()?;
    
    // MFT cluster at offset 0x30
    cursor.seek(SeekFrom::Start(0x30)).ok()?;
    let mft_cluster = cursor.read_u64::<LittleEndian>().ok()?;
    
    // MFT record size at offset 0x40
    cursor.seek(SeekFrom::Start(0x40)).ok()?;
    let mft_size_raw = cursor.read_i8().ok()?;
    
    let mft_record_size = if mft_size_raw > 0 {
        (mft_size_raw as u32) * (sectors_per_cluster as u32) * (bytes_per_sector as u32)
    } else {
        1u32 << ((-mft_size_raw) as u32)
    };
    
    let cluster_size = (bytes_per_sector as u32) * (sectors_per_cluster as u32);
    
    Some(NtfsBootSector {
        bytes_per_sector,
        sectors_per_cluster,
        mft_cluster,
        mft_record_size,
        cluster_size,
    })
}

/// Parse a single MFT record
pub fn parse_mft_record(data: &[u8], record_number: u64) -> Option<MftEntry> {
    if data.len() < MFT_RECORD_SIZE {
        return None;
    }
    
    // Check signature
    if &data[0..4] != MFT_SIGNATURE {
        // Log non-FILE signatures for debugging (might be deleted/zeroed records)
        if record_number % 10000 == 0 {
            let sig = String::from_utf8_lossy(&data[0..4]);
            eprintln!("Record {} signature: {:?} (expected FILE)", record_number, sig);
        }
        return None;
    }
    
    let mut cursor = Cursor::new(data);
    
    // Update sequence offset at 0x04
    cursor.seek(SeekFrom::Start(0x04)).ok()?;
    let update_seq_offset = cursor.read_u16::<LittleEndian>().ok()?;
    let update_seq_size = cursor.read_u16::<LittleEndian>().ok()?;
    
    // Apply fixup array
    let mut fixed_data = data.to_vec();
    apply_fixup(&mut fixed_data, update_seq_offset as usize, update_seq_size as usize);
    cursor = Cursor::new(&fixed_data);
    
    // Flags at offset 0x16
    cursor.seek(SeekFrom::Start(0x16)).ok()?;
    let flags = cursor.read_u16::<LittleEndian>().ok()?;
    let is_in_use = (flags & 0x01) != 0;
    let is_directory = (flags & 0x02) != 0;
    
    // First attribute offset at 0x14
    cursor.seek(SeekFrom::Start(0x14)).ok()?;
    let first_attr_offset = cursor.read_u16::<LittleEndian>().ok()?;
    
    let mut file_name = String::new();
    let mut parent_record = 0u64;
    let mut file_size = 0u64;
    let mut allocated_size = 0u64;
    let mut created_time = 0i64;
    let mut modified_time = 0i64;
    let mut accessed_time = 0i64;
    let mut data_runs = Vec::new();
    
    // Parse attributes
    let mut attr_offset = first_attr_offset as usize;
    
    while attr_offset < fixed_data.len() - 4 {
        let attr_type = u32::from_le_bytes([
            fixed_data[attr_offset],
            fixed_data[attr_offset + 1],
            fixed_data[attr_offset + 2],
            fixed_data[attr_offset + 3],
        ]);
        
        if attr_type == ATTRIBUTE_END || attr_type == 0 {
            break;
        }
        
        let attr_length = u32::from_le_bytes([
            fixed_data[attr_offset + 4],
            fixed_data[attr_offset + 5],
            fixed_data[attr_offset + 6],
            fixed_data[attr_offset + 7],
        ]) as usize;
        
        if attr_length == 0 || attr_offset + attr_length > fixed_data.len() {
            break;
        }
        
        match attr_type {
            ATTRIBUTE_STANDARD_INFORMATION => {
                if let Some(times) = parse_standard_info(&fixed_data[attr_offset..attr_offset + attr_length]) {
                    created_time = times.0;
                    modified_time = times.1;
                    accessed_time = times.2;
                }
            }
            ATTRIBUTE_FILE_NAME => {
                if let Some((name, parent, size, alloc)) = parse_file_name_attr(&fixed_data[attr_offset..attr_offset + attr_length]) {
                    if file_name.is_empty() || name.len() > file_name.len() {
                        file_name = name;
                        parent_record = parent;
                        if size > 0 {
                            file_size = size;
                        }
                        if alloc > 0 {
                            allocated_size = alloc;
                        }
                    }
                }
            }
            ATTRIBUTE_DATA => {
                if let Some((size, runs)) = parse_data_attr(&fixed_data[attr_offset..attr_offset + attr_length]) {
                    if size > file_size {
                        file_size = size;
                    }
                    // Only update data_runs if we found some, or if this is the unnamed data stream
                    // Check if this is the unnamed stream (attribute name length = 0 at offset 9)
                    if attr_offset + 9 < fixed_data.len() {
                        let name_length = fixed_data[attr_offset + 9];
                        if name_length == 0 || !runs.is_empty() {
                            // This is the main data stream or we found data runs
                            if runs.len() > data_runs.len() {
                                data_runs = runs;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        
        attr_offset += attr_length;
    }
    
    // Extract extension
    let extension = file_name
        .rsplit('.')
        .next()
        .filter(|ext| ext.len() <= 10 && *ext != file_name)
        .unwrap_or("")
        .to_lowercase();
    
    Some(MftEntry {
        record_number,
        file_name,
        parent_record,
        file_size,
        allocated_size,
        created_time,
        modified_time,
        accessed_time,
        is_deleted: !is_in_use,
        is_directory,
        is_in_use,
        data_runs,
        extension,
    })
}

/// Apply NTFS fixup array to correct sector boundaries
fn apply_fixup(data: &mut [u8], offset: usize, count: usize) {
    if offset + 2 + count * 2 > data.len() {
        return;
    }
    
    let signature = u16::from_le_bytes([data[offset], data[offset + 1]]);
    
    for i in 1..count {
        let fixup_value = u16::from_le_bytes([
            data[offset + i * 2],
            data[offset + i * 2 + 1],
        ]);
        
        let sector_end = i * 512 - 2;
        if sector_end + 1 < data.len() {
            // Verify signature matches
            let current = u16::from_le_bytes([data[sector_end], data[sector_end + 1]]);
            if current == signature {
                data[sector_end] = fixup_value as u8;
                data[sector_end + 1] = (fixup_value >> 8) as u8;
            }
        }
    }
}

fn parse_standard_info(data: &[u8]) -> Option<(i64, i64, i64)> {
    if data.len() < 72 {
        return None;
    }
    
    // Check if resident
    let non_resident = data[8];
    if non_resident != 0 {
        return None;
    }
    
    let content_offset = u16::from_le_bytes([data[20], data[21]]) as usize;
    
    if content_offset + 32 > data.len() {
        return None;
    }
    
    let created = i64::from_le_bytes([
        data[content_offset], data[content_offset + 1],
        data[content_offset + 2], data[content_offset + 3],
        data[content_offset + 4], data[content_offset + 5],
        data[content_offset + 6], data[content_offset + 7],
    ]);
    
    let modified = i64::from_le_bytes([
        data[content_offset + 8], data[content_offset + 9],
        data[content_offset + 10], data[content_offset + 11],
        data[content_offset + 12], data[content_offset + 13],
        data[content_offset + 14], data[content_offset + 15],
    ]);
    
    let accessed = i64::from_le_bytes([
        data[content_offset + 24], data[content_offset + 25],
        data[content_offset + 26], data[content_offset + 27],
        data[content_offset + 28], data[content_offset + 29],
        data[content_offset + 30], data[content_offset + 31],
    ]);
    
    // Convert Windows FILETIME to Unix timestamp
    fn filetime_to_unix(ft: i64) -> i64 {
        if ft <= 0 {
            return 0;
        }
        (ft / 10_000_000) - 11_644_473_600
    }
    
    Some((
        filetime_to_unix(created),
        filetime_to_unix(modified),
        filetime_to_unix(accessed),
    ))
}

fn parse_file_name_attr(data: &[u8]) -> Option<(String, u64, u64, u64)> {
    if data.len() < 90 {
        return None;
    }
    
    // Check if resident
    let non_resident = data[8];
    if non_resident != 0 {
        return None;
    }
    
    let content_offset = u16::from_le_bytes([data[20], data[21]]) as usize;
    let content_length = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
    
    if content_offset + 66 > data.len() {
        return None;
    }
    
    let content = &data[content_offset..];
    
    // Parent directory reference (first 6 bytes of 8)
    let parent_ref = u64::from_le_bytes([
        content[0], content[1], content[2], content[3],
        content[4], content[5], 0, 0,
    ]);
    
    // Allocated size
    let allocated_size = u64::from_le_bytes([
        content[40], content[41], content[42], content[43],
        content[44], content[45], content[46], content[47],
    ]);
    
    // Real size
    let real_size = u64::from_le_bytes([
        content[48], content[49], content[50], content[51],
        content[52], content[53], content[54], content[55],
    ]);
    
    // Filename length
    let name_length = content[64] as usize;
    let name_type = content[65];
    
    // Skip DOS names (type 2)
    if name_type == 2 {
        return None;
    }
    
    if 66 + name_length * 2 > content.len() {
        return None;
    }
    
    // Parse Unicode filename
    let mut name_chars = Vec::with_capacity(name_length);
    for i in 0..name_length {
        let c = u16::from_le_bytes([
            content[66 + i * 2],
            content[66 + i * 2 + 1],
        ]);
        name_chars.push(c);
    }
    
    let file_name = String::from_utf16_lossy(&name_chars);
    
    Some((file_name, parent_ref, real_size, allocated_size))
}

fn parse_data_attr(data: &[u8]) -> Option<(u64, Vec<DataRun>)> {
    if data.len() < 24 {
        return None;
    }
    
    let non_resident = data[8];
    
    if non_resident == 0 {
        // Resident data
        let content_length = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
        return Some((content_length as u64, Vec::new()));
    }
    
    // Non-resident data
    if data.len() < 64 {
        return None;
    }
    
    // Real size at offset 48
    let real_size = u64::from_le_bytes([
        data[48], data[49], data[50], data[51],
        data[52], data[53], data[54], data[55],
    ]);
    
    // Data runs offset at offset 32
    let runs_offset = u16::from_le_bytes([data[32], data[33]]) as usize;
    
    let data_runs = parse_data_runs(&data[runs_offset..]);
    
    Some((real_size, data_runs))
}

fn parse_data_runs(data: &[u8]) -> Vec<DataRun> {
    let mut runs = Vec::new();
    let mut offset = 0;
    let mut prev_cluster: i64 = 0;
    
    while offset < data.len() {
        let header = data[offset];
        if header == 0 {
            break;
        }
        
        let length_size = (header & 0x0F) as usize;
        let offset_size = ((header >> 4) & 0x0F) as usize;
        
        if offset + 1 + length_size + offset_size > data.len() {
            break;
        }
        
        // Parse cluster count
        let mut cluster_count: u64 = 0;
        for i in 0..length_size {
            cluster_count |= (data[offset + 1 + i] as u64) << (i * 8);
        }
        
        // Parse cluster offset (signed)
        let mut cluster_offset: i64 = 0;
        if offset_size > 0 {
            for i in 0..offset_size {
                cluster_offset |= (data[offset + 1 + length_size + i] as i64) << (i * 8);
            }
            
            // Sign extend
            if offset_size < 8 && (data[offset + length_size + offset_size] & 0x80) != 0 {
                for i in offset_size..8 {
                    cluster_offset |= 0xFFi64 << (i * 8);
                }
            }
            
            cluster_offset += prev_cluster;
            prev_cluster = cluster_offset;
        }
        
        runs.push(DataRun {
            cluster_offset,
            cluster_count,
        });
        
        offset += 1 + length_size + offset_size;
    }
    
    runs
}

/// Scan MFT and return all deleted file entries
pub fn scan_mft_for_deleted(mft_data: &[u8], boot_sector: &NtfsBootSector) -> Vec<MftEntry> {
    let mut deleted_files = Vec::new();
    let record_size = boot_sector.mft_record_size as usize;
    
    if record_size == 0 {
        return deleted_files;
    }
    
    let total_records = mft_data.len() / record_size;
    
    for i in 0..total_records {
        let offset = i * record_size;
        if offset + record_size > mft_data.len() {
            break;
        }
        
        let record_data = &mft_data[offset..offset + record_size];
        
        if let Some(entry) = parse_mft_record(record_data, i as u64) {
            // Only include deleted non-system files
            if entry.is_deleted && !entry.file_name.is_empty() {
                // Skip system files
                if !entry.file_name.starts_with('$') {
                    deleted_files.push(entry);
                }
            }
        }
    }
    
    deleted_files
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_filetime_conversion() {
        // Windows FILETIME for 2020-01-01 00:00:00 UTC
        let ft: i64 = 132224352000000000;
        let unix = (ft / 10_000_000) - 11_644_473_600;
        assert!(unix > 0);
    }
}
