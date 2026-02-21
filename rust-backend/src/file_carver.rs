//! File Signature Carving Engine
//! Deep scans raw disk sectors to find files by their magic byte signatures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Known file signature for carving
#[derive(Clone, Debug)]
pub struct FileSignature {
    pub name: &'static str,
    pub extension: &'static str,
    pub header: &'static [u8],
    pub footer: Option<&'static [u8]>,
    pub max_size: u64,
    pub category: &'static str,
}

/// Result of a carved file
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarvedFile {
    pub sector_offset: u64,
    pub byte_offset: u64,
    pub estimated_size: u64,
    pub file_type: String,
    pub extension: String,
    pub category: String,
    pub confidence: u8,  // 0-100
    pub header_match: String,
}

/// Initialize the signature database with common file types
pub fn get_signatures() -> Vec<FileSignature> {
    vec![
        // Images
        FileSignature {
            name: "JPEG Image",
            extension: "jpg",
            header: &[0xFF, 0xD8, 0xFF],
            footer: Some(&[0xFF, 0xD9]),
            max_size: 50 * 1024 * 1024, // 50MB
            category: "Images",
        },
        FileSignature {
            name: "PNG Image",
            extension: "png",
            header: &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            footer: Some(&[0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82]),
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "GIF Image",
            extension: "gif",
            header: &[0x47, 0x49, 0x46, 0x38],
            footer: Some(&[0x00, 0x3B]),
            max_size: 20 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "BMP Image",
            extension: "bmp",
            header: &[0x42, 0x4D],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "WebP Image",
            extension: "webp",
            header: &[0x52, 0x49, 0x46, 0x46],  // RIFF followed by WEBP
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "TIFF Image",
            extension: "tiff",
            header: &[0x49, 0x49, 0x2A, 0x00],  // Little endian
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "ICO Icon",
            extension: "ico",
            header: &[0x00, 0x00, 0x01, 0x00],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Images",
        },
        
        // Documents
        FileSignature {
            name: "PDF Document",
            extension: "pdf",
            header: &[0x25, 0x50, 0x44, 0x46],  // %PDF
            footer: Some(&[0x25, 0x25, 0x45, 0x4F, 0x46]),  // %%EOF
            max_size: 500 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Microsoft Office (DOCX/XLSX/PPTX)",
            extension: "docx",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP (Office Open XML)
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Microsoft Word (DOC)",
            extension: "doc",
            header: &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Rich Text Format",
            extension: "rtf",
            header: &[0x7B, 0x5C, 0x72, 0x74, 0x66],  // {\rtf
            footer: Some(&[0x7D]),  // }
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        
        // Videos - Note: MP4/MOV handled specially in carve_sector due to variable box size
        FileSignature {
            name: "AVI Video",
            extension: "avi",
            header: &[0x52, 0x49, 0x46, 0x46],  // RIFF
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "MKV Video",
            extension: "mkv",
            header: &[0x1A, 0x45, 0xDF, 0xA3],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        // Note: MP4 and MOV are handled specially in carve_sector via ftyp detection
        FileSignature {
            name: "WMV Video",
            extension: "wmv",
            header: &[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "FLV Video",
            extension: "flv",
            header: &[0x46, 0x4C, 0x56, 0x01],  // FLV
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        
        // Audio
        FileSignature {
            name: "MP3 Audio",
            extension: "mp3",
            header: &[0xFF, 0xFB],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "MP3 with ID3",
            extension: "mp3",
            header: &[0x49, 0x44, 0x33],  // ID3
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "WAV Audio",
            extension: "wav",
            header: &[0x52, 0x49, 0x46, 0x46],  // RIFF
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "FLAC Audio",
            extension: "flac",
            header: &[0x66, 0x4C, 0x61, 0x43],  // fLaC
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "OGG Audio",
            extension: "ogg",
            header: &[0x4F, 0x67, 0x67, 0x53],  // OggS
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "M4A Audio",
            extension: "m4a",
            header: &[0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70],
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "WMA Audio",
            extension: "wma",
            header: &[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Audio",
        },
        
        // Archives
        FileSignature {
            name: "ZIP Archive",
            extension: "zip",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: Some(&[0x50, 0x4B, 0x05, 0x06]),
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "RAR Archive",
            extension: "rar",
            header: &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "7-Zip Archive",
            extension: "7z",
            header: &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "GZIP Archive",
            extension: "gz",
            header: &[0x1F, 0x8B, 0x08],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "TAR Archive",
            extension: "tar",
            header: &[0x75, 0x73, 0x74, 0x61, 0x72],  // ustar at offset 257
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        
        // Executables
        FileSignature {
            name: "Windows Executable",
            extension: "exe",
            header: &[0x4D, 0x5A],  // MZ
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Windows DLL",
            extension: "dll",
            header: &[0x4D, 0x5A],  // MZ
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Executables",
        },
        
        // Database
        FileSignature {
            name: "SQLite Database",
            extension: "sqlite",
            header: &[0x53, 0x51, 0x4C, 0x69, 0x74, 0x65],  // SQLite
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        
        // Email
        FileSignature {
            name: "Outlook PST",
            extension: "pst",
            header: &[0x21, 0x42, 0x44, 0x4E],  // !BDN
            footer: None,
            max_size: 50 * 1024 * 1024 * 1024,
            category: "Email",
        },
        
        // Other
        FileSignature {
            name: "XML Document",
            extension: "xml",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],  // <?xml
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "HTML Document",
            extension: "html",
            header: &[0x3C, 0x21, 0x44, 0x4F, 0x43, 0x54, 0x59, 0x50, 0x45],  // <!DOCTYPE
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        
        // ===== RAW CAMERA FORMATS =====
        FileSignature {
            name: "Canon RAW CR2",
            extension: "cr2",
            header: &[0x49, 0x49, 0x2A, 0x00],  // Same as TIFF but check for CR at offset 8
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Canon RAW CR3",
            extension: "cr3",
            header: &[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70, 0x63, 0x72, 0x78],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Nikon NEF RAW",
            extension: "nef",
            header: &[0x4D, 0x4D, 0x00, 0x2A],  // Big endian TIFF
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Sony ARW RAW",
            extension: "arw",
            header: &[0x49, 0x49, 0x2A, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Adobe DNG RAW",
            extension: "dng",
            header: &[0x49, 0x49, 0x2A, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Fujifilm RAF RAW",
            extension: "raf",
            header: &[0x46, 0x55, 0x4A, 0x49, 0x46, 0x49, 0x4C, 0x4D],  // FUJIFILM
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Olympus ORF RAW",
            extension: "orf",
            header: &[0x49, 0x49, 0x52, 0x4F],  // IIRO
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Panasonic RW2 RAW",
            extension: "rw2",
            header: &[0x49, 0x49, 0x55, 0x00],
            footer: None,
            max_size: 80 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Pentax PEF RAW",
            extension: "pef",
            header: &[0x4D, 0x4D, 0x00, 0x2A],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        
        // ===== MORE IMAGE FORMATS =====
        FileSignature {
            name: "Photoshop PSD",
            extension: "psd",
            header: &[0x38, 0x42, 0x50, 0x53],  // 8BPS
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "GIMP XCF",
            extension: "xcf",
            header: &[0x67, 0x69, 0x6D, 0x70, 0x20, 0x78, 0x63, 0x66],  // gimp xcf
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "SVG Image",
            extension: "svg",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],  // <?xml
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "HEIC Image",
            extension: "heic",
            header: &[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70, 0x68, 0x65, 0x69, 0x63],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "AVIF Image",
            extension: "avif",
            header: &[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70, 0x61, 0x76, 0x69, 0x66],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "JPEG 2000",
            extension: "jp2",
            header: &[0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "TGA Image",
            extension: "tga",
            header: &[0x00, 0x00, 0x02, 0x00, 0x00],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        
        // ===== MORE VIDEO FORMATS =====
        FileSignature {
            name: "WebM Video",
            extension: "webm",
            header: &[0x1A, 0x45, 0xDF, 0xA3],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "3GP Video",
            extension: "3gp",
            header: &[0x00, 0x00, 0x00, 0x14, 0x66, 0x74, 0x79, 0x70, 0x33, 0x67, 0x70],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "MPEG Video",
            extension: "mpg",
            header: &[0x00, 0x00, 0x01, 0xBA],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "VOB Video",
            extension: "vob",
            header: &[0x00, 0x00, 0x01, 0xBA],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "M2TS Video",
            extension: "m2ts",
            header: &[0x47, 0x40],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        
        // ===== MORE AUDIO FORMATS =====
        FileSignature {
            name: "AIFF Audio",
            extension: "aiff",
            header: &[0x46, 0x4F, 0x52, 0x4D],  // FORM
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "APE Audio",
            extension: "ape",
            header: &[0x4D, 0x41, 0x43, 0x20],  // MAC
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "AAC Audio",
            extension: "aac",
            header: &[0xFF, 0xF1],
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "MIDI Audio",
            extension: "mid",
            header: &[0x4D, 0x54, 0x68, 0x64],  // MThd
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "AMR Audio",
            extension: "amr",
            header: &[0x23, 0x21, 0x41, 0x4D, 0x52],  // #!AMR
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Audio",
        },
        
        // ===== EBOOKS & DOCUMENTS =====
        FileSignature {
            name: "EPUB eBook",
            extension: "epub",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP container
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "MOBI eBook",
            extension: "mobi",
            header: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x42, 0x4F, 0x4F, 0x4B, 0x4D, 0x4F, 0x42, 0x49],  // BOOKMOBI at offset 60
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "OpenDocument Text",
            extension: "odt",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "PostScript",
            extension: "ps",
            header: &[0x25, 0x21, 0x50, 0x53],  // %!PS
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "LaTeX Document",
            extension: "tex",
            header: &[0x5C, 0x64, 0x6F, 0x63, 0x75, 0x6D, 0x65, 0x6E, 0x74, 0x63, 0x6C, 0x61, 0x73, 0x73],  // \documentclass
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        
        // ===== FONTS =====
        FileSignature {
            name: "TrueType Font",
            extension: "ttf",
            header: &[0x00, 0x01, 0x00, 0x00],
            footer: None,
            max_size: 20 * 1024 * 1024,
            category: "Fonts",
        },
        FileSignature {
            name: "OpenType Font",
            extension: "otf",
            header: &[0x4F, 0x54, 0x54, 0x4F],  // OTTO
            footer: None,
            max_size: 20 * 1024 * 1024,
            category: "Fonts",
        },
        FileSignature {
            name: "WOFF Font",
            extension: "woff",
            header: &[0x77, 0x4F, 0x46, 0x46],  // wOFF
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Fonts",
        },
        FileSignature {
            name: "WOFF2 Font",
            extension: "woff2",
            header: &[0x77, 0x4F, 0x46, 0x32],  // wOF2
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Fonts",
        },
        
        // ===== DATABASES =====
        FileSignature {
            name: "Microsoft Access MDB",
            extension: "mdb",
            header: &[0x00, 0x01, 0x00, 0x00, 0x53, 0x74, 0x61, 0x6E, 0x64, 0x61, 0x72, 0x64, 0x20, 0x4A, 0x65, 0x74],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "Microsoft Access ACCDB",
            extension: "accdb",
            header: &[0x00, 0x01, 0x00, 0x00, 0x53, 0x74, 0x61, 0x6E, 0x64, 0x61, 0x72, 0x64, 0x20, 0x41, 0x43, 0x45],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "MySQL Database",
            extension: "myd",
            header: &[0xFE, 0x01],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        
        // ===== CAD & 3D =====
        FileSignature {
            name: "AutoCAD DWG",
            extension: "dwg",
            header: &[0x41, 0x43, 0x31, 0x30],  // AC10
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "AutoCAD DXF",
            extension: "dxf",
            header: &[0x30, 0x0A, 0x53, 0x45, 0x43, 0x54, 0x49, 0x4F, 0x4E],  // 0\nSECTION
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "STL 3D Model",
            extension: "stl",
            header: &[0x73, 0x6F, 0x6C, 0x69, 0x64],  // solid (ASCII)
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "OBJ 3D Model",
            extension: "obj",
            header: &[0x23],  // # (comment)
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "FBX 3D Model",
            extension: "fbx",
            header: &[0x4B, 0x61, 0x79, 0x64, 0x61, 0x72, 0x61, 0x20, 0x46, 0x42, 0x58],  // Kaydara FBX
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Blender",
            extension: "blend",
            header: &[0x42, 0x4C, 0x45, 0x4E, 0x44, 0x45, 0x52],  // BLENDER
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "SketchUp",
            extension: "skp",
            header: &[0xFF, 0xFE, 0xFF, 0x0E, 0x53, 0x00, 0x6B, 0x00, 0x65, 0x00, 0x74, 0x00, 0x63, 0x00, 0x68, 0x00, 0x55, 0x00, 0x70, 0x00],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        
        // ===== ADOBE CREATIVE SUITE =====
        FileSignature {
            name: "Adobe Illustrator",
            extension: "ai",
            header: &[0x25, 0x50, 0x44, 0x46],  // %PDF
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Design",
        },
        FileSignature {
            name: "Adobe InDesign",
            extension: "indd",
            header: &[0x06, 0x06, 0xED, 0xF5, 0xD8, 0x1D, 0x46, 0xE5],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Design",
        },
        FileSignature {
            name: "Adobe Premiere",
            extension: "prproj",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],  // <?xml
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Design",
        },
        FileSignature {
            name: "Adobe After Effects",
            extension: "aep",
            header: &[0x00, 0x00, 0x00, 0x14, 0x66, 0x74, 0x79, 0x70],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Design",
        },
        
        // ===== MORE ARCHIVES =====
        FileSignature {
            name: "CAB Archive",
            extension: "cab",
            header: &[0x4D, 0x53, 0x43, 0x46],  // MSCF
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "ISO Image",
            extension: "iso",
            header: &[0x43, 0x44, 0x30, 0x30, 0x31],  // CD001 at offset 32769
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "DMG Disk Image",
            extension: "dmg",
            header: &[0x78, 0x01, 0x73, 0x0D, 0x62, 0x62, 0x60],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "VHD Virtual Disk",
            extension: "vhd",
            header: &[0x63, 0x6F, 0x6E, 0x65, 0x63, 0x74, 0x69, 0x78],  // conectix
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "VMDK Virtual Disk",
            extension: "vmdk",
            header: &[0x4B, 0x44, 0x4D],  // KDM
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        
        // ===== PROGRAMMING/CODE =====
        FileSignature {
            name: "Python Script",
            extension: "py",
            header: &[0x23, 0x21, 0x2F, 0x75, 0x73, 0x72, 0x2F, 0x62, 0x69, 0x6E, 0x2F, 0x70, 0x79, 0x74, 0x68, 0x6F, 0x6E],  // #!/usr/bin/python
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Java Class",
            extension: "class",
            header: &[0xCA, 0xFE, 0xBA, 0xBE],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Java JAR",
            extension: "jar",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Android APK",
            extension: "apk",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Code",
        },
        
        // ===== CRYPTO/SECURITY =====
        FileSignature {
            name: "PGP Public Key",
            extension: "asc",
            header: &[0x2D, 0x2D, 0x2D, 0x2D, 0x2D, 0x42, 0x45, 0x47, 0x49, 0x4E, 0x20, 0x50, 0x47, 0x50],  // -----BEGIN PGP
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "PEM Certificate",
            extension: "pem",
            header: &[0x2D, 0x2D, 0x2D, 0x2D, 0x2D, 0x42, 0x45, 0x47, 0x49, 0x4E],  // -----BEGIN
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "X.509 Certificate",
            extension: "der",
            header: &[0x30, 0x82],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "PKCS#12",
            extension: "p12",
            header: &[0x30, 0x82],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Security",
        },
        
        // ===== SPECIALTY FORMATS =====
        FileSignature {
            name: "Windows Registry",
            extension: "reg",
            header: &[0x57, 0x69, 0x6E, 0x64, 0x6F, 0x77, 0x73, 0x20, 0x52, 0x65, 0x67, 0x69, 0x73, 0x74, 0x72, 0x79],  // Windows Registry
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Shortcut",
            extension: "lnk",
            header: &[0x4C, 0x00, 0x00, 0x00, 0x01, 0x14, 0x02, 0x00],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Help",
            extension: "hlp",
            header: &[0x3F, 0x5F, 0x03, 0x00],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Icon Library",
            extension: "icl",
            header: &[0x00, 0x00, 0x01, 0x00],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Cursor",
            extension: "cur",
            header: &[0x00, 0x00, 0x02, 0x00],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "System",
        },
        
        // ===== SUBTITLES =====
        FileSignature {
            name: "SubRip Subtitle",
            extension: "srt",
            header: &[0x31, 0x0D, 0x0A, 0x30, 0x30, 0x3A, 0x30, 0x30],  // 1\r\n00:00
            footer: None,
            max_size: 5 * 1024 * 1024,
            category: "Subtitles",
        },
        FileSignature {
            name: "VTT Subtitle",
            extension: "vtt",
            header: &[0x57, 0x45, 0x42, 0x56, 0x54, 0x54],  // WEBVTT
            footer: None,
            max_size: 5 * 1024 * 1024,
            category: "Subtitles",
        },
        
        // ===== TORRENT/P2P =====
        FileSignature {
            name: "BitTorrent",
            extension: "torrent",
            header: &[0x64, 0x38, 0x3A, 0x61, 0x6E, 0x6E, 0x6F, 0x75, 0x6E, 0x63, 0x65],  // d8:announce
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "P2P",
        },
        
        // ===== DISK IMAGES =====
        FileSignature {
            name: "VirtualBox VDI",
            extension: "vdi",
            header: &[0x3C, 0x3C, 0x3C, 0x20, 0x4F, 0x72, 0x61, 0x63, 0x6C, 0x65],  // <<< Oracle
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "QEMU QCOW",
            extension: "qcow",
            header: &[0x51, 0x46, 0x49],  // QFI
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        
        // ===== MORE OFFICE FORMATS =====
        FileSignature {
            name: "Excel XLSX",
            extension: "xlsx",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "PowerPoint PPTX",
            extension: "pptx",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "OpenDocument Spreadsheet",
            extension: "ods",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "OpenDocument Presentation",
            extension: "odp",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Apple Pages",
            extension: "pages",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Apple Numbers",
            extension: "numbers",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Apple Keynote",
            extension: "key",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Documents",
        },
        
        // ===== MORE RAW FORMATS =====
        FileSignature {
            name: "Sony SR2 RAW",
            extension: "sr2",
            header: &[0x49, 0x49, 0x2A, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Sony SRF RAW",
            extension: "srf",
            header: &[0x49, 0x49, 0x2A, 0x00],
            footer: None,
            max_size: 80 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Kodak DCR RAW",
            extension: "dcr",
            header: &[0x4D, 0x4D, 0x00, 0x2A],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Minolta MRW RAW",
            extension: "mrw",
            header: &[0x00, 0x4D, 0x52, 0x4D],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Samsung SRW RAW",
            extension: "srw",
            header: &[0x49,0x49, 0x2A, 0x00],
            footer: None,
            max_size: 80 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Epson ERF RAW",
            extension: "erf",
            header: &[0x49, 0x49, 0x2A, 0x00],
            footer: None,
            max_size: 80 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Mamiya MEF RAW",
            extension: "mef",
            header: &[0x4D, 0x4D, 0x00, 0x2A],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Leaf MOS RAW",
            extension: "mos",
            header: &[0x4D, 0x4D, 0x00, 0x2A],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Phase One IIQ RAW",
            extension: "iiq",
            header: &[0x49, 0x49, 0x2A, 0x00],
            footer: None,
            max_size: 300 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "Hasselblad 3FR RAW",
            extension: "3fr",
            header: &[0x4D, 0x4D, 0x00, 0x2A],
            footer: None,
            max_size: 150 * 1024 * 1024,
            category: "RAW Photos",
        },
        FileSignature {
            name: "RED R3D RAW",
            extension: "r3d",
            header: &[0x52, 0x45, 0x44, 0x31],  // RED1
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "RAW Photos",
        },
        
        // ===== MORE VIDEO CODECS =====
        FileSignature {
            name: "FLV Video",
            extension: "flv",
            header: &[0x46, 0x4C, 0x56],  // FLV
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "SWF Flash",
            extension: "swf",
            header: &[0x46, 0x57, 0x53],  // FWS
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "Matroska MKV",
            extension: "mkv",
            header: &[0x1A, 0x45, 0xDF, 0xA3],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "OGG Video",
            extension: "ogv",
            header: &[0x4F, 0x67, 0x67, 0x53],  // OggS
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "DivX",
            extension: "divx",
            header: &[0x52, 0x49, 0x46, 0x46],  // RIFF
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "ASF Video",
            extension: "asf",
            header: &[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "MJPEG Video",
            extension: "mjpeg",
            header: &[0xFF, 0xD8, 0xFF],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "MTS Video",
            extension: "mts",
            header: &[0x47],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "TS Transport Stream",
            extension: "ts",
            header: &[0x47, 0x40],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "ProRes Video",
            extension: "prores",
            header: &[0x00, 0x00, 0x00, 0x14, 0x66, 0x74, 0x79, 0x70],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        
        // ===== MORE AUDIO CODECS =====
        FileSignature {
            name: "OGG Audio",
            extension: "ogg",
            header: &[0x4F, 0x67, 0x67, 0x53],  // OggS
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "Opus Audio",
            extension: "opus",
            header: &[0x4F, 0x67, 0x67, 0x53],  // OggS
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "DSD Audio",
            extension: "dsd",
            header: &[0x44, 0x53, 0x44, 0x20],  // DSD
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "ALAC Audio",
            extension: "m4a",
            header: &[0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70, 0x4D, 0x34, 0x41],
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "TTA Audio",
            extension: "tta",
            header: &[0x54, 0x54, 0x41, 0x31],  // TTA1
            footer: None,
            max_size: 300 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "WavPack",
            extension: "wv",
            header: &[0x77, 0x76, 0x70, 0x6B],  // wvpk
            footer: None,
            max_size: 300 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "Shorten Audio",
            extension: "shn",
            header: &[0x61, 0x6A, 0x6B, 0x67],
            footer: None,
            max_size: 300 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "AU Audio",
            extension: "au",
            header: &[0x2E, 0x73, 0x6E, 0x64],  // .snd
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "VOC Audio",
            extension: "voc",
            header: &[0x43, 0x72, 0x65, 0x61, 0x74, 0x69, 0x76, 0x65],  // Creative
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "DSS Audio",
            extension: "dss",
            header: &[0x02, 0x64, 0x73, 0x73],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Audio",
        },
        
        // ===== MORE ARCHIVES =====
        FileSignature {
            name: "TAR Archive",
            extension: "tar",
            header: &[0x75, 0x73, 0x74, 0x61, 0x72],  // ustar (at offset 257)
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "GZIP Archive",
            extension: "gz",
            header: &[0x1F, 0x8B, 0x08],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "BZIP2 Archive",
            extension: "bz2",
            header: &[0x42, 0x5A, 0x68],  // BZh
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "XZ Archive",
            extension: "xz",
            header: &[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "LZH Archive",
            extension: "lzh",
            header: &[0x2D, 0x6C, 0x68],  // -lh
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "ARJ Archive",
            extension: "arj",
            header: &[0x60, 0xEA],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "ACE Archive",
            extension: "ace",
            header: &[0x2A, 0x2A, 0x41, 0x43, 0x45, 0x2A, 0x2A],  // **ACE**
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "StuffIt Archive",
            extension: "sit",
            header: &[0x53, 0x49, 0x54, 0x21],  // SIT!
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "ZPAQ Archive",
            extension: "zpaq",
            header: &[0x7A, 0x50, 0x51],  // zPQ
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "PAK Archive",
            extension: "pak",
            header: &[0x50, 0x41, 0x43, 0x4B],  // PACK
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        
        // ===== MORE EXECUTABLE FORMATS =====
        FileSignature {
            name: "Linux ELF",
            extension: "elf",
            header: &[0x7F, 0x45, 0x4C, 0x46],  // .ELF
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Mach-O Binary",
            extension: "macho",
            header: &[0xFE, 0xED, 0xFA, 0xCE],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Mach-O 64-bit",
            extension: "macho64",
            header: &[0xFE, 0xED, 0xFA, 0xCF],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Windows Batch",
            extension: "bat",
            header: &[0x40, 0x65, 0x63, 0x68, 0x6F],  // @echo
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "PowerShell Script",
            extension: "ps1",
            header: &[0x23, 0x20],  // # (comment)
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Shell Script",
            extension: "sh",
            header: &[0x23, 0x21, 0x2F, 0x62, 0x69, 0x6E, 0x2F, 0x73, 0x68],  // #!/bin/sh
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Executables",
        },
        
        // ===== MORE DATABASE FORMATS =====
        FileSignature {
            name: "MongoDB BSON",
            extension: "bson",
            header: &[0x00, 0x00, 0x00],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "dBASE",
            extension: "dbf",
            header: &[0x03],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "FoxPro",
            extension: "fpt",
            header: &[0x00, 0x00, 0x03, 0x00],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "PostgreSQL Dump",
            extension: "dump",
            header: &[0x50, 0x47, 0x44, 0x4D, 0x50],  // PGDMP
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "Oracle Tablespace",
            extension: "dbf",
            header: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        
        // ===== MORE 3D FORMATS =====
        FileSignature {
            name: "3DS Max",
            extension: "3ds",
            header: &[0x4D, 0x4D],  // MM
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Collada DAE",
            extension: "dae",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],  // <?xml
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Cinema 4D",
            extension: "c4d",
            header: &[0x43, 0x00, 0x34, 0x00, 0x44, 0x00],  // C.4.D.
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Maya Binary",
            extension: "mb",
            header: &[0x46, 0x4F, 0x52, 0x34],  // FOR4
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Maya ASCII",
            extension: "ma",
            header: &[0x2F, 0x2F, 0x4D, 0x61, 0x79, 0x61],  // //Maya
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "LightWave Object",
            extension: "lwo",
            header: &[0x46, 0x4F, 0x52, 0x4D],  // FORM
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Modo Mesh",
            extension: "lxo",
            header: &[0x46, 0x4F, 0x52, 0x4D],  // FORM
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "USD 3D",
            extension: "usd",
            header: &[0x50, 0x53, 0x44],  // PSD
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "glTF Binary",
            extension: "glb",
            header: &[0x67, 0x6C, 0x54, 0x46],  // glTF
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "X3D Model",
            extension: "x3d",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],  // <?xml
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        
        // ===== GIS & MAPPING =====
        FileSignature {
            name: "Shapefile",
            extension: "shp",
            header: &[0x00, 0x00, 0x27, 0x0A],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "GIS",
        },
        FileSignature {
            name: "KML",
            extension: "kml",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],  // <?xml
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "GIS",
        },
        FileSignature {
            name: "KMZ",
            extension: "kmz",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "GIS",
        },
        FileSignature {
            name: "GeoJSON",
            extension: "geojson",
            header: &[0x7B, 0x22, 0x74, 0x79, 0x70, 0x65, 0x22],  // {"type"
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "GIS",
        },
        FileSignature {
            name: "GeoTIFF",
            extension: "tif",
            header: &[0x49, 0x49, 0x2A, 0x00],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "GIS",
        },
        FileSignature {
            name: "MBTiles",
            extension: "mbtiles",
            header: &[0x53, 0x51, 0x4C, 0x69, 0x74, 0x65],  // SQLite
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "GIS",
        },
        FileSignature {
            name: "GPX",
            extension: "gpx",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],  // <?xml
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "GIS",
        },
        
        // ===== GAME FILES =====
        FileSignature {
            name: "Unity Asset Bundle",
            extension: "unity3d",
            header: &[0x55, 0x6E, 0x69, 0x74, 0x79, 0x57, 0x65, 0x62],  // UnityWeb
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Games",
        },
        FileSignature {
            name: "Unreal Package",
            extension: "upk",
            header: &[0xC1, 0x83, 0x2A, 0x9E],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Games",
        },
        FileSignature {
            name: "Source Engine BSP",
            extension: "bsp",
            header: &[0x56, 0x42, 0x53, 0x50],  // VBSP
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Games",
        },
        FileSignature {
            name: "Quake PAK",
            extension: "pak",
            header: &[0x50, 0x41, 0x43, 0x4B],  // PACK
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Games",
        },
        FileSignature {
            name: "WAD Archive",
            extension: "wad",
            header: &[0x49, 0x57, 0x41, 0x44],  // IWAD
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Games",
        },
        FileSignature {
            name: "ROM Image",
            extension: "rom",
            header: &[0x4E, 0x45, 0x53, 0x1A],  // NES.
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Games",
        },
        FileSignature {
            name: "Game Boy ROM",
            extension: "gb",
            header: &[0xCE, 0xED, 0x66, 0x66],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Games",
        },
        FileSignature {
            name: "Nintendo DS ROM",
            extension: "nds",
            header: &[0x2E, 0x00, 0x00, 0xEA],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Games",
        },
        FileSignature {
            name: "PSP ISO",
            extension: "cso",
            header: &[0x43, 0x49, 0x53, 0x4F],  // CISO
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Games",
        },
        
        // ===== SCIENTIFIC DATA =====
        FileSignature {
            name: "HDF5",
            extension: "h5",
            header: &[0x89, 0x48, 0x44, 0x46, 0x0D, 0x0A, 0x1A, 0x0A],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        FileSignature {
            name: "NetCDF",
            extension: "nc",
            header: &[0x43, 0x44, 0x46, 0x01],  // CDF.
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        FileSignature {
            name: "FITS",
            extension: "fits",
            header: &[0x53, 0x49, 0x4D, 0x50, 0x4C, 0x45],  // SIMPLE
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        FileSignature {
            name: "DICOM Medical",
            extension: "dcm",
            header: &[0x44, 0x49, 0x43, 0x4D],  // DICM at offset 128
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        FileSignature {
            name: "NIfTI Neuroimaging",
            extension: "nii",
            header: &[0x6E, 0x69, 0x31, 0x00],  // ni1.
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        FileSignature {
            name: "MATLAB",
            extension: "mat",
            header: &[0x4D, 0x41, 0x54, 0x4C, 0x41, 0x42],  // MATLAB
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        FileSignature {
            name: "R Data",
            extension: "rdata",
            header: &[0x52, 0x44, 0x58, 0x32],  // RDX2
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        FileSignature {
            name: "SAS Data",
            extension: "sas7bdat",
            header: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        FileSignature {
            name: "SPSS Data",
            extension: "sav",
            header: &[0x24, 0x46, 0x4C, 0x32],  // $FL2
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        FileSignature {
            name: "Stata Data",
            extension: "dta",
            header: &[0x3C, 0x73, 0x74, 0x61, 0x74, 0x61],  // <stata
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Scientific",
        },
        
        // ===== MORE IMAGE FORMATS =====
        FileSignature {
            name: "WebP Image",
            extension: "webp",
            header: &[0x52, 0x49, 0x46, 0x46],  // RIFF (WEBP at offset 8)
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "DDS Texture",
            extension: "dds",
            header: &[0x44, 0x44, 0x53, 0x20],  // DDS
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "EXR Image",
            extension: "exr",
            header: &[0x76, 0x2F, 0x31, 0x01],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "HDR Image",
            extension: "hdr",
            header: &[0x23, 0x3F, 0x52, 0x41, 0x44, 0x49, 0x41, 0x4E, 0x43, 0x45],  // #?RADIANCE
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "PCX Image",
            extension: "pcx",
            header: &[0x0A, 0x05, 0x01, 0x01],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Krita",
            extension: "kra",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Affinity Photo",
            extension: "afphoto",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Affinity Designer",
            extension: "afdesign",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Paint.NET",
            extension: "pdn",
            header: &[0x50, 0x44, 0x4E, 0x33],  // PDN3
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Sketch Design",
            extension: "sketch",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Design",
        },
        FileSignature {
            name: "Figma",
            extension: "fig",
            header: &[0x7B, 0x22, 0x64, 0x6F, 0x63, 0x75, 0x6D, 0x65, 0x6E, 0x74],  // {"document
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Design",
        },
        FileSignature {
            name: "CorelDRAW",
            extension: "cdr",
            header: &[0x52, 0x49, 0x46, 0x46],  // RIFF
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Design",
        },
        
        // ===== MORE EBOOK FORMATS =====
        FileSignature {
            name: "AZW3 Kindle",
            extension: "azw3",
            header: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x42, 0x4F, 0x4F, 0x4B, 0x4D, 0x4F, 0x42, 0x49],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "CBR Comic",
            extension: "cbr",
            header: &[0x52, 0x61, 0x72, 0x21],  // Rar!
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "CBZ Comic",
            extension: "cbz",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "FB2 eBook",
            extension: "fb2",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],  // <?xml
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "LIT eBook",
            extension: "lit",
            header: &[0x49, 0x54, 0x4F, 0x4C, 0x49, 0x54, 0x4C, 0x53],  // ITOLITLS
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "PRC Palm",
            extension: "prc",
            header: &[0x42, 0x4F, 0x4F, 0x4B, 0x4D, 0x4F, 0x42, 0x49],  // BOOKMOBI
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        
        // ===== CONFIG & DATA FILES =====
        FileSignature {
            name: "JSON",
            extension: "json",
            header: &[0x7B],  // {
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Data",
        },
        FileSignature {
            name: "YAML",
            extension: "yaml",
            header: &[0x2D, 0x2D, 0x2D],  // ---
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Data",
        },
        FileSignature {
            name: "TOML",
            extension: "toml",
            header: &[0x5B],  // [
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Data",
        },
        FileSignature {
            name: "Protobuf",
            extension: "pb",
            header: &[0x0A],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Data",
        },
        FileSignature {
            name: "MessagePack",
            extension: "msgpack",
            header: &[0x80, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Data",
        },
        FileSignature {
            name: "Apache Avro",
            extension: "avro",
            header: &[0x4F, 0x62, 0x6A, 0x01],  // Obj.
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Data",
        },
        FileSignature {
            name: "Apache Parquet",
            extension: "parquet",
            header: &[0x50, 0x41, 0x52, 0x31],  // PAR1
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Data",
        },
        FileSignature {
            name: "ORC",
            extension: "orc",
            header: &[0x4F, 0x52, 0x43],  // ORC
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Data",
        },
        
        // ===== BLOCKCHAIN & CRYPTO =====
        FileSignature {
            name: "Bitcoin Wallet",
            extension: "wallet",
            header: &[0x0A, 0x16, 0x6F, 0x72, 0x67],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Crypto",
        },
        FileSignature {
            name: "Ethereum Keystore",
            extension: "keystore",
            header: &[0x7B, 0x22, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6F, 0x6E, 0x22],  // {"version"
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Crypto",
        },
        
        // ===== CAD ADDITIONAL =====
        FileSignature {
            name: "Revit",
            extension: "rvt",
            header: &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "SolidWorks Part",
            extension: "sldprt",
            header: &[0xD0, 0xCF, 0x11, 0xE0],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "SolidWorks Assembly",
            extension: "sldasm",
            header: &[0xD0, 0xCF, 0x11, 0xE0],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "CATIA",
            extension: "catpart",
            header: &[0x56, 0x35, 0x5F, 0x43, 0x46, 0x56, 0x32],  // V5_CFV2
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "Inventor Part",
            extension: "ipt",
            header: &[0xD0, 0xCF, 0x11, 0xE0],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "Rhino 3D",
            extension: "3dm",
            header: &[0x33, 0x64, 0x4D],  // 3dM
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "CAD",
        },
        
        // ===== ADDITIONAL FONTS =====
        FileSignature {
            name: "EOT Font",
            extension: "eot",
            header: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4C, 0x50],  // LP at offset 34
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Fonts",
        },
        FileSignature {
            name: "Type 1 Font",
            extension: "pfb",
            header: &[0x80, 0x01],
            footer: None,
            max_size: 5 * 1024 * 1024,
            category: "Fonts",
        },
        FileSignature {
            name: "Printer Font Metrics",
            extension: "pfm",
            header: &[0x00, 0x01],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Fonts",
        },
        
        // ===== MAIL & CALENDAR =====
        FileSignature {
            name: "Outlook OST",
            extension: "ost",
            header: &[0x21, 0x42, 0x44, 0x4E],  // !BDN
            footer: None,
            max_size: 50 * 1024 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "Outlook MSG",
            extension: "msg",
            header: &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "EML Email",
            extension: "eml",
            header: &[0x46, 0x72, 0x6F, 0x6D, 0x20, 0x20, 0x20],  // From
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "vCard",
            extension: "vcf",
            header: &[0x42, 0x45, 0x47, 0x49, 0x4E, 0x3A, 0x56, 0x43, 0x41, 0x52, 0x44],  // BEGIN:VCARD
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Contacts",
        },
        FileSignature {
            name: "iCalendar",
            extension: "ics",
            header: &[0x42, 0x45, 0x47, 0x49, 0x4E, 0x3A, 0x56, 0x43, 0x41, 0x4C, 0x45, 0x4E, 0x44, 0x41, 0x52],  // BEGIN:VCALENDAR
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Calendar",
        },
        
        // ===== ADDITIONAL SYSTEM FILES =====
        FileSignature {
            name: "Windows Prefetch",
            extension: "pf",
            header: &[0x4D, 0x41, 0x4D],  // MAM
            footer: None,
            max_size: 5 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Event Log",
            extension: "evtx",
            header: &[0x45, 0x6C, 0x66, 0x46, 0x69, 0x6C, 0x65],  // ElfFile
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "macOS DMG (old)",
            extension: "dmg",
            header: &[0x6B, 0x6F, 0x6C, 0x79],  // koly
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Linux RPM",
            extension: "rpm",
            header: &[0xED, 0xAB, 0xEE, 0xDB],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Debian DEB",
            extension: "deb",
            header: &[0x21, 0x3C, 0x61, 0x72, 0x63, 0x68, 0x3E],  // !<arch>
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "System",
        },
        
        // ===== E-LEARNING & PRESENTATIONS =====
        FileSignature {
            name: "SCORM Package",
            extension: "scorm",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "E-Learning",
        },
        FileSignature {
            name: "Articulate Storyline",
            extension: "story",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "E-Learning",
        },
        FileSignature {
            name: "Captivate",
            extension: "cptx",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "E-Learning",
        },
        
        // ===== ADDITIONAL MISCELLANEOUS =====
        FileSignature {
            name: "CHM Help",
            extension: "chm",
            header: &[0x49, 0x54, 0x53, 0x46],  // ITSF
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "OneNote",
            extension: "one",
            header: &[0xE4, 0x52, 0x5C, 0x7B, 0x8C, 0xD8, 0xA7, 0x4D],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Evernote ENEX",
            extension: "enex",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],  // <?xml
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Markdown",
            extension: "md",
            header: &[0x23],  // #
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "AsciiDoc",
            extension: "adoc",
            header: &[0x3D],  // =
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "reStructuredText",
            extension: "rst",
            header: &[0x2E, 0x2E],  // ..
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Org Mode",
            extension: "org",
            header: &[0x23, 0x2B, 0x54, 0x49, 0x54, 0x4C, 0x45],  // #+TITLE
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Documents",
        },
        
        // ===== ADDITIONAL VIDEO FORMATS =====
        FileSignature {
            name: "Quicktime MOV",
            extension: "mov",
            header: &[0x00, 0x00, 0x00, 0x14, 0x66, 0x74, 0x79, 0x70],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "MP4 Video Alt",
            extension: "mp4",
            header: &[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70, 0x6D, 0x70, 0x34, 0x32],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "M4V Video",
            extension: "m4v",
            header: &[0x00, 0x00, 0x00, 0x1C, 0x66, 0x74, 0x79, 0x70, 0x4D, 0x34, 0x56],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "MPEG-4 Part 14",
            extension: "m4p",
            header: &[0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70, 0x4D, 0x34, 0x50],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "RealMedia",
            extension: "rm",
            header: &[0x2E, 0x52, 0x4D, 0x46],  // .RMF
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "RealVideo",
            extension: "rv",
            header: &[0x2E, 0x52, 0x4D, 0x46],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "Ogg Theora",
            extension: "ogm",
            header: &[0x4F, 0x67, 0x67, 0x53],  // OggS
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "VP8 Video",
            extension: "ivf",
            header: &[0x44, 0x4B, 0x49, 0x46],  // DKIF
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {name: "H.264 Elementary Stream",
            extension: "h264",
            header: &[0x00, 0x00, 0x00, 0x01],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        FileSignature {
            name: "H.265/HEVC Stream",
            extension: "h265",
            header: &[0x00, 0x00, 0x00, 0x01],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Videos",
        },
        
        // ===== ADDITIONAL AUDIO EXTENDED =====
        FileSignature {
            name: "Real Audio",
            extension: "ra",
            header: &[0x2E, 0x72, 0x61, 0xFD],  // .ra.
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "Adaptive Multi-Rate NB",
            extension: "3ga",
            header: &[0x00, 0x00, 0x00, 0x14, 0x66, 0x74, 0x79, 0x70, 0x33, 0x67, 0x70],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "WebM Audio",
            extension: "weba",
            header: &[0x1A, 0x45, 0xDF, 0xA3],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "PCM Audio",
            extension: "pcm",
            header: &[0x52, 0x49, 0x46, 0x46],  // RIFF
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Audio",
        },
        FileSignature {
            name: "Sony ATRAC",
            extension: "aa3",
            header: &[0x65, 0x61, 0x33],  // ea3
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Audio",
        },
        
        // ===== MORE EXECUTABLE & BINARY =====
        FileSignature {
            name: "DOS Executable",
            extension: "com",
            header: &[0x4D, 0x5A],  // MZ
            footer: None,
            max_size: 64 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Windows Screensaver",
            extension: "scr",
            header: &[0x4D, 0x5A],  // MZ
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Windows Installer",
            extension: "msi",
            header: &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Visual Studio Solution",
            extension: "sln",
            header: &[0xEF, 0xBB, 0xBF, 0x4D, 0x69, 0x63, 0x72, 0x6F, 0x73, 0x6F, 0x66, 0x74],  // BOM + Microsoft
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Ruby Script",
            extension: "rb",
            header: &[0x23, 0x21, 0x2F, 0x75, 0x73, 0x72, 0x2F, 0x62, 0x69, 0x6E, 0x2F, 0x72, 0x75, 0x62, 0x79],  // #!/usr/bin/ruby
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Perl Script",
            extension: "pl",
            header: &[0x23, 0x21, 0x2F, 0x75, 0x73, 0x72, 0x2F, 0x62, 0x69, 0x6E, 0x2F, 0x70, 0x65, 0x72, 0x6C],  // #!/usr/bin/perl
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "PHP Script",
            extension: "php",
            header: &[0x3C, 0x3F, 0x70, 0x68, 0x70],  // <?php
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Swift Source",
            extension: "swift",
            header: &[0x69, 0x6D, 0x70, 0x6F, 0x72, 0x74],  // import
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Go Source",
            extension: "go",
            header: &[0x70, 0x61, 0x63, 0x6B, 0x61, 0x67, 0x65],  // package
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Rust Source",
            extension: "rs",
            header: &[0x75, 0x73, 0x65, 0x20],  // use (common start)
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "TypeScript",
            extension: "ts",
            header: &[0x69, 0x6D, 0x70, 0x6F, 0x72, 0x74],  // import
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Dart",
            extension: "dart",
            header: &[0x69, 0x6D, 0x70, 0x6F, 0x72, 0x74],  // import
            footer: None,
            max_size: 5 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Kotlin",
            extension: "kt",
            header: &[0x70, 0x61, 0x63, 0x6B, 0x61, 0x67, 0x65],  // package
            footer: None,
            max_size: 5 * 1024 * 1024,
            category: "Code",
        },
        
        // ===== MORE ARCHIVE FORMATS =====
        FileSignature {
            name: "LZMA Archive",
            extension: "lzma",
            header: &[0x5D, 0x00, 0x00],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "LZ4 Archive",
            extension: "lz4",
            header: &[0x04, 0x22, 0x4D, 0x18],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "Zstandard",
            extension: "zst",
            header: &[0x28, 0xB5, 0x2F, 0xFD],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "Snappy",
            extension: "sz",
            header: &[0xFF, 0x06, 0x00, 0x00, 0x73, 0x4E, 0x61, 0x50, 0x70, 0x59],  // sNaPpY
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "Brotli",
            extension: "br",
            header: &[0xCE, 0xB2, 0xCF, 0x81],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "Unix Compress",
            extension: "z",
            header: &[0x1F, 0x9D],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "Pack200",
            extension: "pack",
            header: &[0xCA, 0xFE, 0xD0, 0x0D],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "Squashfs",
            extension: "sqsh",
            header: &[0x68, 0x73, 0x71, 0x73],  // hsqs
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "CPIO Archive",
            extension: "cpio",
            header: &[0x30, 0x37, 0x30, 0x37, 0x30, 0x31],  // 070701
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "AR Archive",
            extension: "ar",
            header: &[0x21, 0x3C, 0x61, 0x72, 0x63, 0x68, 0x3E],  // !<arch>
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        
        // ===== MORE IMAGES =====
        FileSignature {
            name: "JPEG XL",
            extension: "jxl",
            header: &[0x00, 0x00, 0x00, 0x0C, 0x4A, 0x58, 0x4C, 0x20],  // JXL
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "JPEG-LS",
            extension: "jls",
            header: &[0xFF, 0xD8, 0xFF, 0xF7],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "PBM Portable Bitmap",
            extension: "pbm",
            header: &[0x50, 0x31, 0x0A],  // P1\n
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "PGM Portable Graymap",
            extension: "pgm",
            header: &[0x50, 0x35, 0x0A],  // P5\n
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "PPM Portable Pixmap",
            extension: "ppm",
            header: &[0x50, 0x36, 0x0A],  // P6\n
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "PAM Portable Arbitrary Map",
            extension: "pam",
            header: &[0x50, 0x37, 0x0A],  // P7\n
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Radiance RGBE",
            extension: "pic",
            header: &[0x23, 0x3F, 0x52, 0x41, 0x44, 0x49, 0x41, 0x4E, 0x43, 0x45],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Truevision TGA",
            extension: "vda",
            header: &[0x00, 0x00, 0x0A, 0x00, 0x00],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "SGI Image",
            extension: "sgi",
            header: &[0x01, 0xDA],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Sun Raster",
            extension: "ras",
            header: &[0x59, 0xA6, 0x6A, 0x95],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "CALS Raster",
            extension: "cal",
            header: &[0x73, 0x72, 0x63, 0x64, 0x6F, 0x63, 0x69, 0x64],  // srcdocid
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Corel Paintbrush",
            extension: "cpx",
            header: &[0x43, 0x50, 0x43, 0x48],  // CPCH
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "ZSoft Paintbrush",
            extension: "dcx",
            header: &[0xB1, 0x68, 0xDE, 0x3A],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Netpbm Format",
            extension: "pnm",
            header: &[0x50, 0x34, 0x0A],  // P4\n (can be P1-P6)
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Quite OK Image",
            extension: "qoi",
            header: &[0x71, 0x6F, 0x69, 0x66],  // qoif
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Adaptive Scalable Texture Compression",
            extension: "astc",
            header: &[0x13, 0xAB, 0xA1, 0x5C],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "KTX Texture",
            extension: "ktx",
            header: &[0xAB, 0x4B, 0x54, 0x58, 0x20, 0x31, 0x31],  // KTX 11
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        FileSignature {
            name: "Basis Universal",
            extension: "basis",
            header: &[0x73, 0x42, 0x41, 0x53],  // sBAS
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Images",
        },
        
        // ===== MORE DOCUMENTS =====
        FileSignature {
            name: "WordPerfect",
            extension: "wpd",
            header: &[0xFF, 0x57, 0x50, 0x43],  // .WPC
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Lotus 1-2-3",
            extension: "wk1",
            header: &[0x00, 0x00, 0x02, 0x00],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Quattro Pro",
            extension: "qpw",
            header: &[0x00, 0x00, 0x02, 0x00],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Microsoft Works",
            extension: "wps",
            header: &[0xD0, 0xCF, 0x11, 0xE0],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Microsoft Publisher",
            extension: "pub",
            header: &[0xD0, 0xCF, 0x11, 0xE0],
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Microsoft Visio",
            extension: "vsd",
            header: &[0xD0, 0xCF, 0x11, 0xE0],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Microsoft Project",
            extension: "mpp",
            header: &[0xD0, 0xCF, 0x11, 0xE0],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "OpenOffice Writer",
            extension: "sxw",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "OpenOffice Calc",
            extension: "sxc",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "OpenOffice Impress",
            extension: "sxi",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "StarOffice",
            extension: "sdw",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Microsoft Write",
            extension: "wri",
            header: &[0x31, 0xBE, 0x00, 0x00],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Ami Pro",
            extension: "sam",
            header: &[0x5B, 0x76, 0x65, 0x72],  // [ver
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        
        // ===== MORE CAD FILES =====
        FileSignature {
            name: "STEP 3D",
            extension: "stp",
            header: &[0x49, 0x53, 0x4F, 0x2D, 0x31, 0x30, 0x33, 0x30, 0x33, 0x2D, 0x32, 0x31],  // ISO-10303-21
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "IGES",
            extension: "igs",
            header: &[0x53, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x31, 0x50],  // "S" + spaces + "1P"
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "Parasolid",
            extension: "x_t",
            header: &[0x2A, 0x2A, 0x53, 0x43, 0x48, 0x45, 0x4D, 0x41],  // **SCHEMA
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "JT Open CAD",
            extension: "jt",
            header: &[0x56, 0x65, 0x72, 0x73, 0x69, 0x6F, 0x6E],  // Version
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "NX Part",
            extension: "prt",
            header: &[0x55, 0x6E, 0x69, 0x67, 0x72, 0x61, 0x70, 0x68, 0x69, 0x63, 0x73],  // Unigraphics
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "CAD",
        },
        FileSignature {
            name: "ACIS SAT",
            extension: "sat",
            header: &[0x34, 0x30, 0x30, 0x20, 0x30, 0x20],  // 400 0
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "CAD",
        },
        
        // ===== MORE 3D FORMATS =====
        FileSignature {
            name: "Alembic",
            extension: "abc",
            header: &[0x4F, 0x67, 0x67, 0x53],  // OggS
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "PLY 3D",
            extension: "ply",
            header: &[0x70, 0x6C, 0x79, 0x0A],  // ply\n
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "OFF 3D",
            extension: "off",
            header: &[0x4F, 0x46, 0x46, 0x0A],  // OFF\n
            footer: None,
            max_size: 200 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "DirectX X",
            extension: "x",
            header: &[0x78, 0x6F, 0x66, 0x20],  // xof
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "AC3D",
            extension: "ac",
            header: &[0x41, 0x43, 0x33, 0x44],  // AC3D
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Valve Model",
            extension: "mdl",
            header: &[0x49, 0x44, 0x53, 0x54],  // IDST
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Quake MD2",
            extension: "md2",
            header: &[0x49, 0x44, 0x50, 0x32],  // IDP2
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Quake MD3",
            extension: "md3",
            header: &[0x49, 0x44, 0x50, 0x33],  // IDP3
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "3D",
        },
        FileSignature {
            name: "Wavefront MTL",
            extension: "mtl",
            header: &[0x23, 0x20],  // # (comment)
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "3D",
        },
        
        // ===== MORE DATABASE FORMATS =====
        FileSignature {
            name: "Microsoft SQL Database",
            extension: "mdf",
            header: &[0x01, 0x0F, 0x00, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "SQL Server Log",
            extension: "ldf",
            header: &[0x01, 0x0F, 0x00, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "InterBase Database",
            extension: "gdb",
            header: &[0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "FileMaker Pro",
            extension: "fp7",
            header: &[0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "Redis Dump",
            extension: "rdb",
            header: &[0x52, 0x45, 0x44, 0x49, 0x53],  // REDIS
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "LevelDB",
            extension: "ldb",
            header: &[0x6C, 0x65, 0x76, 0x65, 0x6C, 0x64, 0x62],  // leveldb
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        FileSignature {
            name: "Berkeley DB",
            extension: "db",
            header: &[0x00, 0x05, 0x31, 0x62],  // Berkeley DB btree
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Databases",
        },
        
        // ===== CONTAINER FORMATS =====
        FileSignature {
            name: "Docker Image",
            extension: "dockerimage",
            header: &[0x7B, 0x22, 0x63, 0x6F, 0x6E, 0x66, 0x69, 0x67],  // {"config
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Containers",
        },
        FileSignature {
            name: "OCI Image",
            extension: "oci",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Containers",
        },
        
        // ===== FIRMWARE & BIOS =====
        FileSignature {
            name: "UEFI Firmware",
            extension: "efi",
            header: &[0x4D, 0x5A],  // MZ
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Firmware",
        },
        FileSignature {
            name: "BIOS ROM",
            extension: "rom",
            header: &[0x55, 0xAA],
            footer: None,
            max_size: 16 * 1024 * 1024,
            category: "Firmware",
        },
        FileSignature {
            name: "Intel HEX",
            extension: "hex",
            header: &[0x3A, 0x31, 0x30],  // :10
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Firmware",
        },
        FileSignature {
            name: "Motorola S-Record",
            extension: "s19",
            header: &[0x53, 0x30],  // S0
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Firmware",
        },
        
        // ===== MORE E-BOOK FORMATS =====
        FileSignature {
            name: "Kindle AZW",
            extension: "azw",
            header: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x54, 0x50, 0x5A],  // TPZ at offset 60
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "iBooks Author",
            extension: "ibooks",
            header: &[0x50, 0x4B, 0x03, 0x04],  // ZIP
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "TCR eBook",
            extension: "tcr",
            header: &[0x5A, 0x42, 0x33],  // ZB3
            footer: None,
            max_size: 20 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "PDB Palm Database",
            extension: "pdb",
            header: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Documents",
        },
        
        // ===== MORE FONT FORMATS =====
        FileSignature {
            name: "Bitmap Font",
            extension: "fnt",
            header: &[0x4D, 0x5A],  // MZ for Windows fonts
            footer: None,
            max_size: 5 * 1024 * 1024,
            category: "Fonts",
        },
        FileSignature {
            name: "X11 Bitmap Distribution Format",
            extension: "bdf",
            header: &[0x53, 0x54, 0x41, 0x52, 0x54, 0x46, 0x4F, 0x4E, 0x54],  // STARTFONT
            footer: None,
            max_size: 5 *1024 * 1024,
            category: "Fonts",
        },
        FileSignature {
            name: "PostScript Type 1",
            extension: "pfa",
            header: &[0x25, 0x21, 0x50, 0x53, 0x2D, 0x41, 0x64, 0x6F, 0x62, 0x65, 0x46, 0x6F, 0x6E, 0x74],  // %!PS-AdobeFont
            footer: None,
            max_size: 5 * 1024 * 1024,
            category: "Fonts",
        },
        
        // ===== SPECIALIZED FORMATS =====
        FileSignature {
            name: "PCAP Packet Capture",
            extension: "pcap",
            header: &[0xD4, 0xC3, 0xB2, 0xA1],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Network",
        },
        FileSignature {
            name: "PCAPNG",
            extension: "pcapng",
            header: &[0x0A, 0x0D, 0x0D, 0x0A],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Network",
        },
        FileSignature {
            name: "Wireshark Capture",
            extension: "snoop",
            header: &[0x73, 0x6E, 0x6F, 0x6F, 0x70],  // snoop
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Network",
        },
        FileSignature {
            name: "Network Monitor Capture",
            extension: "cap",
            header: &[0x52, 0x54, 0x53, 0x53],  // RTSS
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Network",
        },
        FileSignature {
            name: "Core Dump",
            extension: "core",
            header: &[0x7F, 0x45, 0x4C, 0x46],  // ELF
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Minidump",
            extension: "dmp",
            header: &[0x4D, 0x44, 0x4D, 0x50, 0x93, 0xA7],  // MDMP..
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Memory Dump",
            extension: "hdmp",
            header: &[0x50, 0x41, 0x47, 0x45, 0x44, 0x55],  // PAGEDU
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        
        // Additional Filesystem Types
        FileSignature {
            name: "ext2/3/4 Filesystem",
            extension: "ext4",
            header: &[0x53, 0xEF],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "XFS Filesystem",
            extension: "xfs",
            header: &[0x58, 0x46, 0x53, 0x42],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Btrfs Filesystem",
            extension: "btrfs",
            header: &[0x5F, 0x42, 0x48, 0x52, 0x66, 0x53, 0x5F, 0x4D],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "ZFS Filesystem",
            extension: "zfs",
            header: &[0x00, 0x00, 0x00, 0x00, 0x00, 0xBA, 0xB1, 0x0C],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "ReiserFS",
            extension: "reiserfs",
            header: &[0x52, 0x65, 0x49, 0x73, 0x45, 0x72, 0x46, 0x73],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "JFS Filesystem",
            extension: "jfs",
            header: &[0x4A, 0x46, 0x53, 0x31],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "F2FS Filesystem",
            extension: "f2fs",
            header: &[0x10, 0x20, 0xF5, 0xF2],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "APFS Filesystem",
            extension: "apfs",
            header: &[0x4E, 0x58, 0x53, 0x42],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "HFS+ Filesystem",
            extension: "hfsplus",
            header: &[0x48, 0x2B, 0x00, 0x04],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "FAT12 Filesystem",
            extension: "fat12",
            header: &[0xEB, 0x3C, 0x90],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "FAT16 Filesystem",
            extension: "fat16",
            header: &[0xEB, 0x52, 0x90],
            footer: None,
            max_size: 50 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "FAT32 Filesystem",
            extension: "fat32",
            header: &[0xEB, 0x58, 0x90],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "exFAT Filesystem",
            extension: "exfat",
            header: &[0xEB, 0x76, 0x90],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "UDF Filesystem",
            extension: "udf",
            header: &[0x00, 0x42, 0x45, 0x41, 0x30, 0x31],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "NTFS MFT",
            extension: "mft",
            header: &[0x46, 0x49, 0x4C, 0x45],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "System",
        },
        
        // Encryption & Security
        FileSignature {
            name: "LUKS Encrypted Volume",
            extension: "luks",
            header: &[0x4C, 0x55, 0x4B, 0x53, 0xBA, 0xBE],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Encryption",
        },
        FileSignature {
            name: "VeraCrypt Volume",
            extension: "vc",
            header: &[0x56, 0x45, 0x52, 0x41],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Encryption",
        },
        FileSignature {
            name: "TrueCrypt Volume",
            extension: "tc",
            header: &[0x54, 0x52, 0x55, 0x45],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Encryption",
        },
        FileSignature {
            name: "BitLocker Volume",
            extension: "bde",
            header: &[0x2D, 0x46, 0x56, 0x45, 0x2D, 0x46, 0x53, 0x2D],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Encryption",
        },
        FileSignature {
            name: "PGP Encrypted Message",
            extension: "pgp",
            header: &[0x2D, 0x2D, 0x2D, 0x2D, 0x2D, 0x42, 0x45, 0x47, 0x49, 0x4E, 0x20, 0x50, 0x47, 0x50],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Encryption",
        },
        FileSignature {
            name: "GnuPG Keyring",
            extension: "gpg",
            header: &[0x99, 0x01],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Encryption",
        },
        FileSignature {
            name: "SSH Private Key",
            extension: "pem",
            header: &[0x2D, 0x2D, 0x2D, 0x2D, 0x2D, 0x42, 0x45, 0x47, 0x49, 0x4E, 0x20, 0x4F, 0x50, 0x45, 0x4E, 0x53, 0x53, 0x48],
            footer: None,
            max_size: 10 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "Java Keystore",
            extension: "jks",
            header: &[0xFE, 0xED, 0xFE, 0xED],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "PKCS#7 Certificate",
            extension: "p7b",
            header: &[0x30, 0x82],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Security",
        },
        
        // Backup Formats
        FileSignature {
            name: "Acronis True Image",
            extension: "tib",
            header: &[0xB4, 0x6E, 0x68, 0x44],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Backup",
        },
        FileSignature {
            name: "Macrium Reflect",
            extension: "mrimg",
            header: &[0x4D, 0x52, 0x49, 0x4D, 0x47],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Backup",
        },
        FileSignature {
            name: "Veeam Backup",
            extension: "vbk",
            header: &[0x56, 0x45, 0x45, 0x41, 0x4D],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Backup",
        },
        FileSignature {
            name: "Norton Ghost",
            extension: "gho",
            header: &[0xFE, 0xEF, 0x00, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Backup",
        },
        FileSignature {
            name: "Windows Backup",
            extension: "bkf",
            header: &[0x54, 0x41, 0x50, 0x45],
            footer: None,
            max_size: 50 * 1024 * 1024 * 1024,
            category: "Backup",
        },
        
        // Forensic Image Formats
        FileSignature {
            name: "EnCase Evidence File",
            extension: "e01",
            header: &[0x45, 0x56, 0x46, 0x09, 0x0D, 0x0A, 0xFF, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Forensics",
        },
        FileSignature {
            name: "FTK Imager",
            extension: "ad1",
            header: &[0x41, 0x44, 0x31, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Forensics",
        },
        FileSignature {
            name: "AFF Forensic Format",
            extension: "aff",
            header: &[0x41, 0x46, 0x46, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Forensics",
        },
        FileSignature {
            name: "Raw DD Image",
            extension: "dd",
            header: &[0x00, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Forensics",
        },
        
        // Archive Formats
        FileSignature {
            name: "PKZIP Multi-Volume",
            extension: "z01",
            header: &[0x50, 0x4B, 0x07, 0x08],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "Split RAR Archive",
            extension: "r00",
            header: &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "PAR2 Recovery",
            extension: "par2",
            header: &[0x50, 0x41, 0x52, 0x32],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Archives",
        },
        FileSignature {
            name: "UUEncode",
            extension: "uue",
            header: &[0x62, 0x65, 0x67, 0x69, 0x6E, 0x20],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Archives",
        },
        
        // Email & Communication
        FileSignature {
            name: "mbox Format",
            extension: "mbox",
            header: &[0x46, 0x72, 0x6F, 0x6D, 0x20],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "MIME Message",
            extension: "mime",
            header: &[0x2D, 0x2D, 0x2D, 0x2D, 0x2D, 0x2D],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "TNEF Attachment",
            extension: "dat",
            header: &[0x78, 0x9F, 0x3E, 0x22],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "S/MIME Message",
            extension: "p7m",
            header: &[0x30, 0x82],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Email",
        },
        
        // Development & Build Tools
        FileSignature {
            name: "LLVM Bitcode",
            extension: "bc",
            header: &[0x42, 0x43, 0xC0, 0xDE],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "WebAssembly Binary",
            extension: "wasm",
            header: &[0x00, 0x61, 0x73, 0x6D],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Jupyter Notebook",
            extension: "ipynb",
            header: &[0x7B, 0x0A, 0x20, 0x22, 0x63, 0x65, 0x6C, 0x6C, 0x73, 0x22],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Code",
        },
        
        // Package Formats
        FileSignature {
            name: "NuGet Package",
            extension: "nupkg",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Python Wheel",
            extension: "whl",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Python Egg",
            extension: "egg",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "RubyGem Package",
            extension: "gem",
            header: &[0x1F, 0x8B, 0x08],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Code",
        },
        
        // Application Packages
        FileSignature {
            name: "iOS App Package",
            extension: "ipa",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Android App Bundle",
            extension: "aab",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 2 * 1024 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "MSIX Package",
            extension: "msix",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 5 * 1024 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Chrome Extension",
            extension: "crx",
            header: &[0x43, 0x72, 0x32, 0x34],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Firefox Extension",
            extension: "xpi",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Web App Archive",
            extension: "war",
            header: &[0x50, 0x4B, 0x03, 0x04],
            footer: None,
            max_size: 500 * 1024 * 1024,
            category: "Executables",
        },
        
        // System Components
        FileSignature {
            name: "Windows Driver",
            extension: "sys",
            header: &[0x4D, 0x5A],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "Linux Kernel Module",
            extension: "ko",
            header: &[0x7F, 0x45, 0x4C, 0x46],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "ActiveX Control",
            extension: "ocx",
            header: &[0x4D, 0x5A],
            footer: None,
            max_size: 50 * 1024 * 1024,
            category: "Executables",
        },
        FileSignature {
            name: "COM Component",
            extension: "cpl",
            header: &[0x4D, 0x5A],
            footer: None,
            max_size: 20 * 1024 * 1024,
            category: "Executables",
        },
        
        // Network & Remote Access
        FileSignature {
            name: "PCAP Network Capture",
            extension: "pcap",
            header: &[0xD4, 0xC3, 0xB2, 0xA1],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Network",
        },
        FileSignature {
            name: "PCAPNG Capture",
            extension: "pcapng",
            header: &[0x0A, 0x0D, 0x0D, 0x0A],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Network",
        },
        FileSignature {
            name: "RDP Session File",
            extension: "rdp",
            header: &[0x73, 0x63, 0x72, 0x65, 0x65, 0x6E, 0x20, 0x6D, 0x6F, 0x64, 0x65],
            footer: None,
            max_size: 100 * 1024,
            category: "Network",
        },
        FileSignature {
            name: "VNC Session",
            extension: "vnc",
            header: &[0x52, 0x46, 0x42, 0x20],
            footer: None,
            max_size: 10 * 1024,
            category: "Network",
        },
        
        // Document Formats
        FileSignature {
            name: "R Markdown",
            extension: "rmd",
            header: &[0x2D, 0x2D, 0x2D, 0x0A, 0x74, 0x69, 0x74, 0x6C, 0x65],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Documents",
        },
        FileSignature {
            name: "Quarto Document",
            extension: "qmd",
            header: &[0x2D, 0x2D, 0x2D, 0x0A, 0x74, 0x69, 0x74, 0x6C, 0x65],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Documents",
        },
        
        // System State
        FileSignature {
            name: "Windows Hibernation",
            extension: "hiberfil",
            header: &[0x68, 0x69, 0x62, 0x72],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Page File",
            extension: "pagefile",
            header: &[0x00, 0x00, 0x00, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Linux Swap",
            extension: "swap",
            header: &[0x53, 0x57, 0x41, 0x50, 0x53, 0x50, 0x41, 0x43, 0x45, 0x32],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "System",
        },
        
        // Container & Virtualization
        FileSignature {
            name: "VMware VMDK",
            extension: "vmdk",
            header: &[0x4B, 0x44, 0x4D],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "VirtualBox VDI",
            extension: "vdi",
            header: &[0x3C, 0x3C, 0x3C, 0x20, 0x4F, 0x72, 0x61, 0x63, 0x6C, 0x65],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "QEMU QCOW2",
            extension: "qcow2",
            header: &[0x51, 0x46, 0x49, 0xFB],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "Docker Image",
            extension: "docker",
            header: &[0x7B, 0x22, 0x61, 0x75, 0x74, 0x68, 0x73, 0x22],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Containers",
        },
        
        // Authentication & Credentials
        FileSignature {
            name: "X11 Authority",
            extension: "xauth",
            header: &[0x01, 0x00],
            footer: None,
            max_size: 100 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "Kerberos Ticket",
            extension: "ccache",
            header: &[0x05, 0x04],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "AWS Credentials",
            extension: "credentials",
            header: &[0x5B, 0x64, 0x65, 0x66, 0x61, 0x75, 0x6C, 0x74, 0x5D],
            footer: None,
            max_size: 100 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "SSH Public Key",
            extension: "pub",
            header: &[0x73, 0x73, 0x68, 0x2D],
            footer: None,
            max_size: 10 * 1024,
            category: "Security",
        },
        
        // Data Exchange
        FileSignature {
            name: "LDAP Data",
            extension: "ldif",
            header: &[0x76, 0x65, 0x72, 0x73, 0x69, 0x6F, 0x6E, 0x3A],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Data",
        },
        FileSignature {
            name: "vCard Contact",
            extension: "vcf",
            header: &[0x42, 0x45, 0x47, 0x49, 0x4E, 0x3A, 0x56, 0x43, 0x41, 0x52, 0x44],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Contacts",
        },
        FileSignature {
            name: "iCalendar",
            extension: "ics",
            header: &[0x42, 0x45, 0x47, 0x49, 0x4E, 0x3A, 0x56, 0x43, 0x41, 0x4C, 0x45, 0x4E, 0x44, 0x41, 0x52],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Calendar",
        },
        
        // Additional Specialized Formats
        FileSignature {
            name: "Outlook OST File",
            extension: "ost",
            header: &[0x21, 0x42, 0x44, 0x4E],
            footer: None,
            max_size: 50 * 1024 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "Outlook MSG File",
            extension: "msg",
            header: &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "EML Email",
            extension: "eml",
            header: &[0x46, 0x72, 0x6F, 0x6D],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "Thunderbird MBOX",
            extension: "mbox",
            header: &[0x46, 0x72, 0x6F, 0x6D, 0x20, 0x2D],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Email",
        },
        FileSignature {
            name: "Windows Prefetch",
            extension: "pf",
            header: &[0x53, 0x43, 0x43, 0x41],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Event Log",
            extension: "evtx",
            header: &[0x45, 0x6C, 0x66, 0x46, 0x69, 0x6C, 0x65],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Registry Hive",
            extension: "dat",
            header: &[0x72, 0x65, 0x67, 0x66],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Windows Shortcut",
            extension: "lnk",
            header: &[0x4C, 0x00, 0x00, 0x00, 0x01, 0x14, 0x02],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "System",
        },
        FileSignature {
            name: "Virtual Machine Snapshot",
            extension: "vmsn",
            header: &[0xD0, 0xBE, 0xD0, 0xBE],
            footer: None,
            max_size: 50 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "Hyper-V Disk",
            extension: "vhd",
            header: &[0x63, 0x6F, 0x6E, 0x65, 0x63, 0x74, 0x69, 0x78],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "Hyper-V VHDX",
            extension: "vhdx",
            header: &[0x76, 0x68, 0x64, 0x78, 0x66, 0x69, 0x6C, 0x65],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "Parallels Disk",
            extension: "hdd",
            header: &[0x57, 0x69, 0x74, 0x68, 0x6F, 0x75, 0x74, 0x20, 0x66, 0x72, 0x65, 0x65],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "Apple Disk Image",
            extension: "dmg",
            header: &[0x78, 0x01, 0x73, 0x0D, 0x62, 0x62, 0x60],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "Toast Disk Image",
            extension: "toast",
            header: &[0x45, 0x52, 0x02, 0x00, 0x00, 0x00],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Disk Images",
        },
        FileSignature {
            name: "eCryptfs Encrypted",
            extension: "ecryptfs",
            header: &[0x3A, 0xFE, 0x00, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Encryption",
        },
        FileSignature {
            name: "FileVault Encrypted",
            extension: "sparsebundle",
            header: &[0x00, 0x05, 0x16, 0x07, 0x00, 0x02, 0x00, 0x00],
            footer: None,
            max_size: 100 * 1024 * 1024 * 1024,
            category: "Encryption",
        },
        FileSignature {
            name: "GnuPG Public Key",
            extension: "asc",
            header: &[0x2D, 0x2D, 0x2D, 0x2D, 0x2D, 0x42, 0x45, 0x47, 0x49, 0x4E],
            footer: None,
            max_size: 100 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "OpenSSL Certificate",
            extension: "crt",
            header: &[0x2D, 0x2D, 0x2D, 0x2D, 0x2D, 0x42, 0x45, 0x47, 0x49, 0x4E, 0x20, 0x43, 0x45, 0x52, 0x54],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "JWT Token",
            extension: "jwt",
            header: &[0x65, 0x79, 0x4A],
            footer: None,
            max_size: 100 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "OAuth Token",
            extension: "oauth",
            header: &[0x7B, 0x22, 0x61, 0x63, 0x63, 0x65, 0x73, 0x73, 0x5F, 0x74, 0x6F, 0x6B, 0x65, 0x6E],
            footer: None,
            max_size: 10 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "SAML Metadata",
            extension: "saml",
            header: &[0x3C, 0x6D, 0x64, 0x3A, 0x45, 0x6E, 0x74, 0x69, 0x74, 0x79, 0x44, 0x65, 0x73, 0x63],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "API Key File",
            extension: "apikey",
            header: &[0x42, 0x45, 0x41, 0x52, 0x45, 0x52],
            footer: None,
            max_size: 10 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "Kubernetes Config",
            extension: "kubeconfig",
            header: &[0x61, 0x70, 0x69, 0x56, 0x65, 0x72, 0x73, 0x69, 0x6F, 0x6E],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Security",
        },
        FileSignature {
            name: "Docker Compose",
            extension: "docker-compose",
            header: &[0x76, 0x65, 0x72, 0x73, 0x69, 0x6F, 0x6E, 0x3A],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Containers",
        },
        FileSignature {
            name: "Terraform State",
            extension: "tfstate",
            header: &[0x7B, 0x0A, 0x20, 0x20, 0x22, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6F, 0x6E],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Ansible Playbook",
            extension: "ansible",
            header: &[0x2D, 0x2D, 0x2D, 0x0A],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Git Pack File",
            extension: "pack",
            header: &[0x50, 0x41, 0x43, 0x4B],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Git Index",
            extension: "idx",
            header: &[0xFF, 0x74, 0x4F, 0x63],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Subversion DB",
            extension: "svn-base",
            header: &[0x53, 0x51, 0x4C, 0x69, 0x74, 0x65],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Mercurial Store",
            extension: "hg",
            header: &[0x00, 0x01, 0x00, 0x01],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "CVS Repository",
            extension: "cvs",
            header: &[0x43, 0x56, 0x53, 0x20, 0x52, 0x65, 0x70, 0x6F],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Perforce Depot",
            extension: "p4d",
            header: &[0x00, 0x00, 0x01, 0x00],
            footer: None,
            max_size: 10 * 1024 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "TFS Workspace",
            extension: "tfs",
            header: &[0x54, 0x46, 0x53, 0x57, 0x4F, 0x52, 0x4B],
            footer: None,
            max_size: 1 * 1024 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "NPM Shrinkwrap",
            extension: "npm-shrinkwrap",
            header: &[0x7B, 0x0A, 0x20, 0x20, 0x22, 0x6E, 0x61, 0x6D, 0x65],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Yarn Lock",
            extension: "yarn.lock",
            header: &[0x23, 0x20, 0x54, 0x48, 0x49, 0x53],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Composer Lock",
            extension: "composer.lock",
            header: &[0x7B, 0x0A, 0x20, 0x20, 0x20, 0x20, 0x22, 0x5F, 0x72, 0x65, 0x61, 0x64, 0x6D, 0x65],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Pipfile Lock",
            extension: "Pipfile.lock",
            header: &[0x7B, 0x0A, 0x20, 0x20, 0x20, 0x20, 0x22, 0x5F, 0x6D, 0x65, 0x74, 0x61],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Poetry Lock",
            extension: "poetry.lock",
            header: &[0x23, 0x20, 0x54, 0x68, 0x69, 0x73, 0x20, 0x66, 0x69, 0x6C, 0x65],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Gradle Wrapper",
            extension: "gradle-wrapper",
            header: &[0x64, 0x69, 0x73, 0x74, 0x72, 0x69, 0x62, 0x75, 0x74, 0x69, 0x6F, 0x6E],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Maven Settings",
            extension: "settings.xml",
            header: &[0x3C, 0x73, 0x65, 0x74, 0x74, 0x69, 0x6E, 0x67, 0x73],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Cargo Lock",
            extension: "Cargo.lock",
            header: &[0x23, 0x20, 0x54, 0x68, 0x69, 0x73, 0x20, 0x66, 0x69, 0x6C, 0x65],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Go Sum",
            extension: "go.sum",
            header: &[0x67, 0x6F, 0x6C, 0x61, 0x6E, 0x67, 0x2E, 0x6F, 0x72, 0x67],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Gemfile Lock",
            extension: "Gemfile.lock",
            header: &[0x47, 0x45, 0x4D, 0x0A, 0x20, 0x20, 0x72, 0x65, 0x6D, 0x6F, 0x74, 0x65],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Bundle Config",
            extension: "bundleconfig",
            header: &[0x2D, 0x2D, 0x2D, 0x0A, 0x42, 0x55, 0x4E, 0x44, 0x4C, 0x45],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Stack YAML",
            extension: "stack.yaml",
            header: &[0x72, 0x65, 0x73, 0x6F, 0x6C, 0x76, 0x65, 0x72],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Cabal Project",
            extension: "cabal",
            header: &[0x6E, 0x61, 0x6D, 0x65, 0x3A],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Mix Lock",
            extension: "mix.lock",
            header: &[0x25, 0x7B, 0x22],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Rebar Lock",
            extension: "rebar.lock",
            header: &[0x7B, 0x22, 0x31, 0x2E, 0x31, 0x2E, 0x30, 0x22],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Pub Lock",
            extension: "pubspec.lock",
            header: &[0x23, 0x20, 0x47, 0x65, 0x6E, 0x65, 0x72, 0x61, 0x74, 0x65, 0x64],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Swift Package",
            extension: "Package.resolved",
            header: &[0x7B, 0x0A, 0x20, 0x20, 0x22, 0x6F, 0x62, 0x6A, 0x65, 0x63, 0x74],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "CocoaPods Lock",
            extension: "Podfile.lock",
            header: &[0x50, 0x4F, 0x44, 0x53, 0x3A, 0x0A],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Carthage Resolved",
            extension: "Cartfile.resolved",
            header: &[0x67, 0x69, 0x74, 0x68, 0x75, 0x62],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "NuGet Config",
            extension: "nuget.config",
            header: &[0x3C, 0x3F, 0x78, 0x6D, 0x6C],
            footer: None,
            max_size: 1 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "MSBuild Targets",
            extension: "targets",
            header: &[0x3C, 0x50, 0x72, 0x6F, 0x6A, 0x65, 0x63, 0x74],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Ant Build",
            extension: "build.xml",
            header: &[0x3C, 0x70, 0x72, 0x6F, 0x6A, 0x65, 0x63, 0x74],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "CMake Cache",
            extension: "CMakeCache.txt",
            header: &[0x23, 0x20, 0x54, 0x68, 0x69, 0x73, 0x20, 0x69, 0x73],
            footer: None,
            max_size: 10 * 1024 * 1024,
            category: "Code",
        },
        FileSignature {
            name: "Ninja Build Log",
            extension: "ninja_log",
            header: &[0x23, 0x20, 0x6E, 0x69, 0x6E, 0x6A, 0x61, 0x20, 0x6C, 0x6F, 0x67],
            footer: None,
            max_size: 100 * 1024 * 1024,
            category: "Code",
        },

    ]
}

/// Build a lookup table for fast signature matching
pub fn build_signature_lookup() -> HashMap<u16, Vec<FileSignature>> {
    let mut lookup: HashMap<u16, Vec<FileSignature>> = HashMap::new();
    
    for sig in get_signatures() {
        if sig.header.len() >= 2 {
            let key = u16::from_le_bytes([sig.header[0], sig.header[1]]);
            lookup.entry(key).or_insert_with(Vec::new).push(sig);
        }
    }
    
    lookup
}

/// Carve files from raw sector data
pub fn carve_sector(
    data: &[u8],
    sector_offset: u64,
    signatures: &HashMap<u16, Vec<FileSignature>>,
) -> Vec<CarvedFile> {
    let mut carved = Vec::new();
    
    if data.len() < 32 {
        return carved;
    }
    
    // Track positions where we've already found files to avoid duplicates
    let mut found_positions: std::collections::HashSet<u64> = std::collections::HashSet::new();
    
    // First, scan for MP4/MOV files by looking for "ftyp" at offset 4
    for i in 4..data.len().saturating_sub(16) {
        if &data[i..i+4] == b"ftyp" {
            // Verify box size at offset i-4
            let box_size = u32::from_be_bytes([data[i-4], data[i-3], data[i-2], data[i-1]]);
            
            // Valid ftyp box size is typically 8-32 bytes
            if box_size >= 8 && box_size <= 64 {
                let file_start = i - 4;
                let global_offset = sector_offset * 512 + file_start as u64;
                
                if found_positions.contains(&global_offset) {
                    continue;
                }
                
                // Verify brand (next 4 bytes after "ftyp")
                let brand = &data[i+4..i+8];
                let is_valid_brand = brand == b"isom" || brand == b"mp41" || brand == b"mp42" ||
                                     brand == b"M4V " || brand == b"qt  " || brand == b"MSNV" ||
                                     brand == b"3gp4" || brand == b"3gp5" || brand == b"avc1" ||
                                     brand == b"M4A " || brand == b"f4v " || brand == b"dash";
                
                if is_valid_brand {
                    found_positions.insert(global_offset);
                    
                    // Try to determine file size from moov/mdat atoms
                    let estimated_size = estimate_mp4_size(&data[file_start..]);
                    
                    carved.push(CarvedFile {
                        sector_offset,
                        byte_offset: file_start as u64,
                        estimated_size,
                        file_type: "MP4 Video".to_string(),
                        extension: "mp4".to_string(),
                        category: "Videos".to_string(),
                        confidence: 95,
                        header_match: hex::encode(&data[file_start..std::cmp::min(file_start + 16, data.len())]),
                    });
                }
            }
        }
    }
    
    // Then scan for other signatures
    for i in 0..data.len().saturating_sub(32) {
        let key = u16::from_le_bytes([data[i], data[i + 1]]);
        
        if let Some(sigs) = signatures.get(&key) {
            for sig in sigs {
                if i + sig.header.len() <= data.len() {
                    // Check if full header matches
                    if data[i..i + sig.header.len()] == *sig.header {
                        // Skip if we already found something at this position
                        let global_offset = sector_offset * 512 + i as u64;
                        if found_positions.contains(&global_offset) {
                            continue;
                        }
                        
                        // Additional validation for specific formats
                        let confidence = validate_signature(sig, &data[i..]);
                        
                        if confidence >= 75 {
                            // Estimate file size
                            let estimated_size = estimate_file_size(sig, &data[i..]);
                            
                            // Skip unreasonably small files
                            if estimated_size < 1024 {
                                continue;
                            }
                            
                            found_positions.insert(global_offset);
                            
                            carved.push(CarvedFile {
                                sector_offset,
                                byte_offset: i as u64,
                                estimated_size,
                                file_type: sig.name.to_string(),
                                extension: sig.extension.to_string(),
                                category: sig.category.to_string(),
                                confidence,
                                header_match: hex::encode(&data[i..std::cmp::min(i + 16, data.len())]),
                            });
                        }
                    }
                }
            }
        }
    }
    
    carved
}

/// Estimate MP4 file size by parsing atoms
fn estimate_mp4_size(data: &[u8]) -> u64 {
    let mut offset = 0usize;
    let mut last_valid_end = 0u64;
    
    while offset + 8 < data.len() {
        let atom_size = u32::from_be_bytes([
            data[offset], data[offset + 1],
            data[offset + 2], data[offset + 3],
        ]) as u64;
        
        // Handle extended size (size = 1 means 64-bit size follows)
        let actual_size = if atom_size == 1 && offset + 16 < data.len() {
            u64::from_be_bytes([
                data[offset + 8], data[offset + 9],
                data[offset + 10], data[offset + 11],
                data[offset + 12], data[offset + 13],
                data[offset + 14], data[offset + 15],
            ])
        } else if atom_size == 0 {
            // Size 0 means atom extends to end of file - use large estimate
            return 100 * 1024 * 1024; // 100MB estimate
        } else {
            atom_size
        };
        
        // Validate atom size
        if actual_size < 8 || actual_size > 50 * 1024 * 1024 * 1024 {
            break;
        }
        
        last_valid_end = offset as u64 + actual_size;
        offset += actual_size as usize;
        
        // Safety limit - don't scan more than 10MB of headers
        if offset > 10 * 1024 * 1024 {
            break;
        }
    }
    
    if last_valid_end > 0 {
        last_valid_end
    } else {
        50 * 1024 * 1024 // 50MB default
    }
}

/// Validate a signature match with additional checks
fn validate_signature(sig: &FileSignature, data: &[u8]) -> u8 {
    let mut confidence: u8 = 70; // Base confidence for header match
    
    match sig.extension {
        "jpg" | "jpeg" => {
            // JPEG should have valid markers
            if data.len() > 3 && data[2] == 0xFF {
                confidence = 90;
                // Check for JFIF or Exif
                if data.len() > 10 {
                    if &data[6..10] == b"JFIF" || &data[6..10] == b"Exif" {
                        confidence = 98;
                    }
                }
            }
        }
        "png" => {
            // PNG has specific chunk structure
            if data.len() > 16 && &data[12..16] == b"IHDR" {
                confidence = 98;
            }
        }
        "pdf" => {
            // PDF version check
            if data.len() > 8 && data[4] == b'-' && data[5].is_ascii_digit() {
                confidence = 95;
            }
        }
        "zip" | "docx" | "xlsx" | "pptx" => {
            // ZIP local file header check
            if data.len() > 30 {
                let compressed_size = u32::from_le_bytes([data[18], data[19], data[20], data[21]]);
                let filename_len = u16::from_le_bytes([data[26], data[27]]);
                
                if filename_len > 0 && filename_len < 256 && compressed_size < 1_000_000_000 {
                    confidence = 90;
                    
                    // Check for Office documents
                    if data.len() > 30 + filename_len as usize {
                        let filename_start = 30;
                        let filename_end = filename_start + filename_len as usize;
                        if filename_end <= data.len() {
                            let filename = String::from_utf8_lossy(&data[filename_start..filename_end]);
                            if filename.contains("[Content_Types].xml") || filename.starts_with("word/") {
                                confidence = 98;
                            }
                        }
                    }
                }
            }
        }
        "mp3" => {
            if data.len() > 3 {
                // ID3 tag validation
                if &data[0..3] == b"ID3" {
                    confidence = 95;
                }
                // MP3 frame sync
                else if data[0] == 0xFF && (data[1] & 0xE0) == 0xE0 {
                    confidence = 85;
                }
            }
        }
        "mp4" | "mov" => {
            // We now match on ftyp directly, so file data starts at box size
            // Check that we have a valid MP4 structure
            if data.len() > 16 {
                // First 4 bytes should be box size, next 4 should be "ftyp"
                let box_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                
                if box_size >= 8 && box_size <= 1024 && &data[4..8] == b"ftyp" {
                    confidence = 95;
                    
                    // Verify brand is one of the known ones
                    if data.len() > 12 {
                        let brand = &data[8..12];
                        if brand == b"isom" || brand == b"mp41" || brand == b"mp42" || 
                           brand == b"M4V " || brand == b"qt  " || brand == b"MSNV" ||
                           brand == b"3gp4" || brand == b"3gp5" || brand == b"avc1" {
                            confidence = 98;
                        }
                    }
                } else {
                    // If structure doesn't match, likely false positive
                    confidence = 40;
                }
            } else {
                confidence = 40;
            }
        }
        "exe" | "dll" => {
            // Check for PE header
            if data.len() > 64 {
                let pe_offset = u32::from_le_bytes([data[60], data[61], data[62], data[63]]) as usize;
                if pe_offset < data.len() - 4 && &data[pe_offset..pe_offset + 4] == b"PE\0\0" {
                    confidence = 95;
                }
            }
        }
        _ => {
            // Default validation - just header match
        }
    }
    
    confidence
}

/// Estimate the size of a carved file
fn estimate_file_size(sig: &FileSignature, data: &[u8]) -> u64 {
    // Try to find footer if available
    if let Some(footer) = sig.footer {
        let max_search = std::cmp::min(data.len(), sig.max_size as usize);
        
        // Search for footer
        for i in sig.header.len()..max_search.saturating_sub(footer.len()) {
            if data[i..i + footer.len()] == *footer {
                return (i + footer.len()) as u64;
            }
        }
    }
    
    // Try format-specific size detection
    match sig.extension {
        "mp4" | "mov" => {
            // MP4/MOV: Try to find mdat atom which contains the actual media data
            // Parse atoms to find total file size
            let mut offset = 0usize;
            let mut total_size = 0u64;
            
            while offset + 8 < data.len() {
                let atom_size = u32::from_be_bytes([
                    data[offset], data[offset + 1],
                    data[offset + 2], data[offset + 3],
                ]) as u64;
                
                // Handle extended size (size = 1 means 64-bit size follows)
                let (actual_size, header_len) = if atom_size == 1 && offset + 16 < data.len() {
                    let ext_size = u64::from_be_bytes([
                        data[offset + 8], data[offset + 9],
                        data[offset + 10], data[offset + 11],
                        data[offset + 12], data[offset + 13],
                        data[offset + 14], data[offset + 15],
                    ]);
                    (ext_size, 16)
                } else if atom_size == 0 {
                    // Size 0 means atom extends to end of file
                    break;
                } else {
                    (atom_size, 8)
                };
                
                // Validate atom size
                if actual_size < 8 || actual_size > 50 * 1024 * 1024 * 1024 {
                    break;
                }
                
                total_size = offset as u64 + actual_size;
                offset += actual_size as usize;
                
                // Safety limit - don't scan more than 1MB of headers
                if offset > 1024 * 1024 {
                    break;
                }
            }
            
            if total_size > 0 && total_size < sig.max_size as u64 {
                return total_size;
            }
        }
        "png" => {
            // PNG chunk-based size calculation
            let mut offset = 8; // Skip header
            while offset + 12 < data.len() {
                let chunk_size = u32::from_be_bytes([
                    data[offset], data[offset + 1],
                    data[offset + 2], data[offset + 3],
                ]) as usize;
                
                let chunk_type = &data[offset + 4..offset + 8];
                
                offset += 12 + chunk_size; // header + data + CRC
                
                if chunk_type == b"IEND" {
                    return offset as u64;
                }
                
                if chunk_size > 100_000_000 {
                    break; // Invalid chunk
                }
            }
        }
        "zip" | "docx" | "xlsx" | "pptx" => {
            // ZIP end of central directory
            let max_search = std::cmp::min(data.len(), 100_000_000);
            let search_start = max_search.saturating_sub(65535 + 22);
            
            for i in (search_start..max_search.saturating_sub(4)).rev() {
                if &data[i..i + 4] == &[0x50, 0x4B, 0x05, 0x06] {
                    // Found EOCD
                    if i + 22 <= data.len() {
                        let comment_len = u16::from_le_bytes([data[i + 20], data[i + 21]]) as usize;
                        return (i + 22 + comment_len) as u64;
                    }
                }
            }
        }
        "bmp" => {
            // BMP file size in header
            if data.len() > 6 {
                let size = u32::from_le_bytes([data[2], data[3], data[4], data[5]]);
                if size > 0 && size < sig.max_size as u32 {
                    return size as u64;
                }
            }
        }
        _ => {}
    }
    
    // Return a reasonable estimate based on typical file sizes
    match sig.extension {
        "jpg" | "jpeg" => 500 * 1024,    // 500KB average
        "png" => 300 * 1024,              // 300KB average
        "gif" => 100 * 1024,              // 100KB average
        "pdf" => 1 * 1024 * 1024,         // 1MB average
        "mp3" => 5 * 1024 * 1024,         // 5MB average
        "mp4" => 50 * 1024 * 1024,        // 50MB average
        "doc" | "docx" => 200 * 1024,     // 200KB average
        _ => 1 * 1024 * 1024,             // 1MB default
    }
}

/// Get statistics about available signatures
pub fn get_signature_stats() -> HashMap<String, usize> {
    let sigs = get_signatures();
    let mut stats: HashMap<String, usize> = HashMap::new();
    
    for sig in sigs {
        *stats.entry(sig.category.to_string()).or_insert(0) += 1;
    }
    
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_jpeg_detection() {
        let jpeg_header = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        let lookup = build_signature_lookup();
        let carved = carve_sector(&jpeg_header, 0, &lookup);
        assert!(!carved.is_empty());
        assert_eq!(carved[0].extension, "jpg");
    }
    
    #[test]
    fn test_png_detection() {
        let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let lookup = build_signature_lookup();
        let carved = carve_sector(&png_header, 0, &lookup);
        assert!(!carved.is_empty());
        assert_eq!(carved[0].extension, "png");
    }
    
    #[test]
    fn test_pdf_detection() {
        let pdf_header = b"%PDF-1.4".to_vec();
        let lookup = build_signature_lookup();
        let carved = carve_sector(&pdf_header, 0, &lookup);
        assert!(!carved.is_empty());
        assert_eq!(carved[0].extension, "pdf");
    }
}
