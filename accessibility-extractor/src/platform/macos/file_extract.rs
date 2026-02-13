//! Direct file extraction for Office and iWork documents.
//!
//! This module extracts content directly from document files without using
//! AppleScript/Automation, avoiding TCC permission prompts. It uses:
//! - `calamine` for Excel files (.xlsx, .xls, .xlsb)
//! - `docx-rs` for Word files (.docx)
//! - Custom XML parsing for PowerPoint files (.pptx)
//! - Custom IWA/Snappy/Protobuf parsing for Apple iWork files (.pages, .numbers, .key)
//! - AXDocument accessibility attribute to get the file path of open documents
//! - Native `proc_pidinfo` API as fallback to discover which files an application has open

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use accessibility::attribute::AXAttribute;
use accessibility::{AXUIElement, AXUIElementAttributes};
use calamine::{open_workbook_auto, Data, Reader};
use core_foundation::base::{CFType, TCFType};
use core_foundation::string::CFString;

use crate::types::{AppSource, ExtractedContent, ExtractionError};

// ============================================================================
// File Extensions
// ============================================================================

/// File extensions supported for direct extraction
const EXCEL_EXTENSIONS: &[&str] = &[".xlsx", ".xls", ".xlsb", ".xlsm", ".ods"];
const WORD_EXTENSIONS: &[&str] = &[".docx"];
const POWERPOINT_EXTENSIONS: &[&str] = &[".pptx"];
const PAGES_EXTENSIONS: &[&str] = &[".pages"];
const NUMBERS_EXTENSIONS: &[&str] = &[".numbers"];
const KEYNOTE_EXTENSIONS: &[&str] = &[".key"];

// ============================================================================
// AXDocument-based File Path Discovery
// ============================================================================

/// Get the document file path from an application using the AXDocument accessibility attribute.
/// 
/// This is the preferred method for getting the open document path because:
/// 1. It uses the existing Accessibility permission (no additional TCC prompts)
/// 2. It works with sandboxed apps like Microsoft Office
/// 3. It's more reliable than proc_pidinfo for modern macOS apps
/// 
/// The AXDocument attribute returns a file:// URL that we convert to a path.
/// 
/// # Arguments
/// * `bundle_id` - The bundle identifier of the application
/// 
/// # Returns
/// * `Ok(PathBuf)` - The file path of the open document
/// * `Err(ExtractionError)` - If the document path cannot be retrieved
pub fn get_document_path_via_ax(bundle_id: &str) -> Result<PathBuf, ExtractionError> {
    log::info!("[AX-DOC] Getting document path for {} via AXDocument attribute", bundle_id);
    
    // Get the application by bundle ID
    let app = AXUIElement::application_with_bundle_timeout(bundle_id, Duration::from_secs(5))
        .map_err(|e| {
            log::error!("[AX-DOC] Failed to get application {}: {:?}", bundle_id, e);
            ExtractionError::AppNotFound(format!("Application not found: {}", bundle_id))
        })?;
    
    // Try to get the focused window first
    let window = match app.focused_window() {
        Ok(w) => {
            log::info!("[AX-DOC] Got focused window");
            w
        }
        Err(e) => {
            log::warn!("[AX-DOC] No focused window (app may not be frontmost): {:?}", e);
            // Fall back to getting the first window from AXWindows
            get_first_window(&app)?
        }
    };
    
    // Query the AXDocument attribute from the window
    let doc_attr = AXAttribute::<CFType>::new(&CFString::new("AXDocument"));
    let doc_value = window.attribute(&doc_attr).map_err(|e| {
        log::warn!("[AX-DOC] AXDocument attribute not available: {:?}", e);
        ExtractionError::NoContentFound("AXDocument attribute not available".into())
    })?;
    
    // Convert CFType to string (should be a file:// URL)
    let type_id = doc_value.type_of();
    if type_id != CFString::type_id() {
        log::error!("[AX-DOC] AXDocument is not a string type");
        return Err(ExtractionError::PlatformError("AXDocument is not a string".into()));
    }
    
    let ptr = doc_value.as_CFTypeRef();
    let cf_string: CFString = unsafe {
        CFString::wrap_under_get_rule(ptr as core_foundation::string::CFStringRef)
    };
    let url_string = cf_string.to_string();
    
    log::info!("[AX-DOC] Got AXDocument URL: {}", url_string);
    
    // Convert file:// URL to path
    let path = file_url_to_path(&url_string)?;
    
    log::info!("[AX-DOC] Converted to path: {:?}", path);
    
    Ok(path)
}

/// Get the first window from an application's AXWindows attribute.
/// This is used as a fallback when the app is not frontmost and focused_window() fails.
fn get_first_window(app: &AXUIElement) -> Result<AXUIElement, ExtractionError> {
    use core_foundation::array::CFArray;
    
    log::info!("[AX-DOC] Getting first window from AXWindows attribute");
    
    let windows_attr = AXAttribute::<CFType>::new(&CFString::new("AXWindows"));
    let windows_value = app.attribute(&windows_attr).map_err(|e| {
        log::error!("[AX-DOC] Failed to get AXWindows: {:?}", e);
        ExtractionError::ElementNotFound(format!("Failed to get windows: {:?}", e))
    })?;
    
    // The value should be a CFArray of AXUIElements
    let type_id = windows_value.type_of();
    let array_type_id = CFArray::<CFType>::type_id();
    
    if type_id != array_type_id {
        log::error!("[AX-DOC] AXWindows is not an array");
        return Err(ExtractionError::PlatformError("AXWindows is not an array".into()));
    }
    
    // Convert to CFArray
    let ptr = windows_value.as_CFTypeRef();
    let windows_array: CFArray<CFType> = unsafe {
        CFArray::wrap_under_get_rule(ptr as core_foundation::array::CFArrayRef)
    };
    
    log::info!("[AX-DOC] Found {} windows", windows_array.len());
    
    if windows_array.is_empty() {
        return Err(ExtractionError::ElementNotFound("No windows found".into()));
    }
    
    // Get the first window
    let first_window = windows_array.get(0).ok_or_else(|| {
        ExtractionError::ElementNotFound("Failed to get first window".into())
    })?;
    
    // Convert CFType to AXUIElement
    let window_type_id = first_window.type_of();
    let ax_type_id = AXUIElement::type_id();
    
    if window_type_id != ax_type_id {
        log::error!("[AX-DOC] Window is not an AXUIElement");
        return Err(ExtractionError::PlatformError("Window is not an AXUIElement".into()));
    }
    
    let window_ptr = first_window.as_CFTypeRef();
    let window = unsafe {
        AXUIElement::wrap_under_get_rule(window_ptr as accessibility_sys::AXUIElementRef)
    };
    
    // Log the window title for debugging
    if let Ok(title) = window.title() {
        log::info!("[AX-DOC] First window title: {}", title);
    }
    
    Ok(window)
}

/// Convert a file:// URL to a filesystem path.
/// 
/// Handles various URL formats:
/// - `file:///path/to/file.xlsx`
/// - `file://localhost/path/to/file.xlsx` (older macOS)
/// - URL-encoded characters like `%20` for spaces
/// 
/// # Arguments
/// * `url` - The file:// URL string
/// 
/// # Returns
/// * `Ok(PathBuf)` - The decoded filesystem path
/// * `Err(ExtractionError)` - If the URL cannot be parsed
fn file_url_to_path(url: &str) -> Result<PathBuf, ExtractionError> {
    // Remove the file:// prefix
    let path_str = if url.starts_with("file://localhost") {
        // Older macOS format: file://localhost/path
        url.strip_prefix("file://localhost")
            .ok_or_else(|| ExtractionError::PlatformError("Invalid file URL".into()))?
    } else if url.starts_with("file://") {
        // Modern format: file:///path (note: three slashes, first two are protocol, third is root)
        url.strip_prefix("file://")
            .ok_or_else(|| ExtractionError::PlatformError("Invalid file URL".into()))?
    } else {
        return Err(ExtractionError::PlatformError(format!("Not a file URL: {}", url)));
    };
    
    // URL decode the path (handle %20 for spaces, etc.)
    let decoded = percent_decode(path_str);
    
    let path = PathBuf::from(&decoded);
    
    if !path.exists() {
        log::warn!("[AX-DOC] Path does not exist: {:?}", path);
        // Don't error here - the file might be on a network drive or the path might still work
    }
    
    Ok(path)
}

/// Decode percent-encoded characters in a URL path.
/// 
/// Common encodings:
/// - `%20` -> space
/// - `%2F` -> /
/// - `%25` -> %
fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            // Try to read two hex digits
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            // If parsing failed, keep the original
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }
    
    result
}

// ============================================================================
// Native Process File Discovery (replaces lsof)
// ============================================================================

/// Get the process ID for an application by bundle ID using native APIs
pub fn get_pid_for_bundle_id(bundle_id: &str) -> Option<u32> {
    // Map bundle IDs to process names
    let process_name = match bundle_id {
        "com.microsoft.Excel" => "Microsoft Excel",
        "com.microsoft.Word" => "Microsoft Word",
        "com.microsoft.Powerpoint" => "Microsoft PowerPoint",
        "com.apple.iWork.Pages" => "Pages",
        "com.apple.iWork.Numbers" => "Numbers",
        "com.apple.iWork.Keynote" => "Keynote",
        _ => return None,
    };

    // Use native proc_listallpids and proc_name to find the process
    find_pid_by_name(process_name)
}

/// Find a process ID by its name using native libproc APIs
fn find_pid_by_name(name: &str) -> Option<u32> {
    use std::ffi::CStr;
    
    // Get list of all PIDs
    let num_pids = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    if num_pids <= 0 {
        return None;
    }
    
    let mut pids: Vec<i32> = vec![0; num_pids as usize];
    let actual = unsafe {
        libc::proc_listallpids(
            pids.as_mut_ptr() as *mut libc::c_void,
            (pids.len() * std::mem::size_of::<i32>()) as i32,
        )
    };
    
    if actual <= 0 {
        return None;
    }
    
    let count = actual as usize / std::mem::size_of::<i32>();
    
    // Search for matching process name
    for &pid in pids.iter().take(count) {
        if pid <= 0 {
            continue;
        }
        
        let mut proc_name = [0u8; 256];
        let len = unsafe {
            libc::proc_name(pid, proc_name.as_mut_ptr() as *mut libc::c_void, 256)
        };
        
        if len > 0 {
            if let Ok(proc_name_str) = CStr::from_bytes_until_nul(&proc_name) {
                if let Ok(s) = proc_name_str.to_str() {
                    if s == name || name.contains(s) || s.contains(name) {
                        return Some(pid as u32);
                    }
                }
            }
        }
    }
    
    None
}

/// Find open files for a process using native proc_pidinfo API
/// This replaces the lsof command for better performance and no subprocess overhead
pub fn find_open_files(pid: u32, extensions: &[&str]) -> Vec<PathBuf> {
    find_open_files_native(pid, extensions)
}

/// Native implementation using proc_pidinfo
fn find_open_files_native(pid: u32, extensions: &[&str]) -> Vec<PathBuf> {
    use std::mem;
    
    // Constants from libproc.h
    const PROC_PIDLISTFDS: i32 = 1;
    const PROX_FDTYPE_VNODE: u32 = 1;
    const PROC_PIDFDVNODEPATHINFO: i32 = 2;
    
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ProcFDInfo {
        proc_fd: i32,
        proc_fdtype: u32,
    }
    
    #[repr(C)]
    struct VnodePathInfo {
        vip_vi: [u8; 152],  // vnode_info_path structure
        vip_path: [u8; 1024], // MAXPATHLEN
    }
    
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    
    // Get the size needed for file descriptor list
    let buffer_size = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            PROC_PIDLISTFDS,
            0,
            std::ptr::null_mut(),
            0,
        )
    };
    
    if buffer_size <= 0 {
        log::warn!("[PROC] Failed to get fd list size for pid {}", pid);
        return files;
    }
    
    // Allocate buffer and get file descriptors
    let num_fds = buffer_size as usize / mem::size_of::<ProcFDInfo>();
    let mut fd_list: Vec<ProcFDInfo> = vec![ProcFDInfo { proc_fd: 0, proc_fdtype: 0 }; num_fds];
    
    let actual_size = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            PROC_PIDLISTFDS,
            0,
            fd_list.as_mut_ptr() as *mut libc::c_void,
            buffer_size,
        )
    };
    
    if actual_size <= 0 {
        log::warn!("[PROC] Failed to get fd list for pid {}", pid);
        return files;
    }
    
    let actual_count = actual_size as usize / mem::size_of::<ProcFDInfo>();
    
    // For each file descriptor, get the path if it's a vnode
    for fd_info in fd_list.iter().take(actual_count) {
        if fd_info.proc_fdtype != PROX_FDTYPE_VNODE {
            continue;
        }
        
        let mut path_info: VnodePathInfo = unsafe { mem::zeroed() };
        
        let result = unsafe {
            libc::proc_pidfdinfo(
                pid as i32,
                fd_info.proc_fd,
                PROC_PIDFDVNODEPATHINFO,
                &mut path_info as *mut VnodePathInfo as *mut libc::c_void,
                mem::size_of::<VnodePathInfo>() as i32,
            )
        };
        
        if result <= 0 {
            continue;
        }
        
        // Extract path from the buffer
        let path_bytes = &path_info.vip_path;
        let path_len = path_bytes.iter().position(|&b| b == 0).unwrap_or(path_bytes.len());
        
        if let Ok(path_str) = std::str::from_utf8(&path_bytes[..path_len]) {
            let path = Path::new(path_str);
            
            // Check if it matches our extensions
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = format!(".{}", ext.to_lowercase());
                if extensions.iter().any(|e| *e == ext_lower) {
                    // Skip temp files and duplicates
                    if !path_str.contains("~$") && !seen.contains(path_str) {
                        seen.insert(path_str.to_string());
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
    }
    
    files
}

// ============================================================================
// Common Utilities
// ============================================================================

/// Copy a file to a temp location to avoid lock issues
fn copy_to_temp(path: &Path) -> Result<PathBuf, ExtractionError> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document");

    let temp_path = std::env::temp_dir().join(format!("ax_extract_{}", file_name));

    std::fs::copy(path, &temp_path).map_err(|e| {
        ExtractionError::PlatformError(format!("Failed to copy file to temp: {}", e))
    })?;

    Ok(temp_path)
}

// ============================================================================
// Excel Extraction (using calamine)
// ============================================================================

/// Format a cell value for TSV output
/// - Numbers are formatted cleanly (no unnecessary decimals)
/// - Strings are escaped if they contain tabs or newlines
/// - Empty cells become empty strings
fn format_cell_value(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => {
            // Escape tabs and newlines in string values to preserve TSV format
            s.replace('\t', "    ").replace('\n', " ").replace('\r', "")
        }
        Data::Float(f) => {
            // Format floats cleanly - no trailing zeros for whole numbers
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{:.0}", f)
            } else if f.abs() < 0.0001 || f.abs() > 1e10 {
                // Use scientific notation for very small or large numbers
                format!("{:.4e}", f)
            } else {
                // Round to reasonable precision
                let rounded = (f * 10000.0).round() / 10000.0;
                format!("{}", rounded)
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Data::Error(e) => format!("#ERR:{:?}", e),
        Data::DateTime(dt) => format!("{}", dt),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
    }
}

/// Extract content from an Excel file using calamine
pub fn extract_excel(path: &Path) -> Result<ExtractedContent, ExtractionError> {
    log::info!("[EXCEL] Extracting from: {:?}", path);
    
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    // Copy to temp to avoid lock issues with open files
    let temp_path = copy_to_temp(path)?;

    let result = extract_excel_from_path(&temp_path, &file_name);

    // Clean up temp file
    if let Err(e) = std::fs::remove_file(&temp_path) {
        log::warn!("[EXCEL] Failed to remove temp file: {}", e);
    }

    result
}

fn extract_excel_from_path(path: &Path, title: &str) -> Result<ExtractedContent, ExtractionError> {
    let mut workbook = open_workbook_auto(path).map_err(|e| {
        ExtractionError::PlatformError(format!("Failed to open workbook: {}", e))
    })?;

    let sheet_names = workbook.sheet_names().to_vec();

    if sheet_names.is_empty() {
        return Err(ExtractionError::NoContentFound(
            "Workbook has no sheets".into(),
        ));
    }

    let mut all_content = Vec::new();

    for (sheet_idx, sheet_name) in sheet_names.iter().enumerate() {
        if let Ok(range) = workbook.worksheet_range(sheet_name) {
            let mut sheet_content = Vec::new();

            for row in range.rows() {
                let row_text: Vec<String> = row
                    .iter()
                    .map(|cell| format_cell_value(cell))
                    .collect();

                // Only include rows that have at least one non-empty cell
                if row_text.iter().any(|s| !s.is_empty()) {
                    sheet_content.push(row_text.join("\t"));
                }
            }

            if !sheet_content.is_empty() {
                // Format sheet with clear header and TSV content
                let sheet_header = format!(
                    "# Sheet {}: {}\n# Rows: {}",
                    sheet_idx + 1,
                    sheet_name,
                    sheet_content.len()
                );
                
                let sheet_block = format!(
                    "{}\n\n{}",
                    sheet_header,
                    sheet_content.join("\n")
                );
                
                all_content.push(sheet_block);
            }
        }
    }

    if all_content.is_empty() {
        return Err(ExtractionError::NoContentFound(
            "All sheets are empty".into(),
        ));
    }
    
    // Join sheets with clear separation
    let content = all_content.join("\n\n---\n\n");

    Ok(ExtractedContent {
        source: AppSource::Excel.as_str().to_string(),
        title: Some(title.to_string()),
        content,
        app_name: "Microsoft Excel".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "calamine".to_string(),
    })
}

// ============================================================================
// Word Extraction (using docx-rs)
// ============================================================================

/// Extract content from a Word document using docx-rs
pub fn extract_word(path: &Path) -> Result<ExtractedContent, ExtractionError> {
    log::info!("[WORD] Extracting from: {:?}", path);
    
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let temp_path = copy_to_temp(path)?;
    let result = extract_word_from_path(&temp_path, &file_name);
    let _ = std::fs::remove_file(&temp_path);

    result
}

fn extract_word_from_path(path: &Path, title: &str) -> Result<ExtractedContent, ExtractionError> {
    let bytes = std::fs::read(path)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to read file: {}", e)))?;

    let docx = docx_rs::read_docx(&bytes)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to parse docx: {:?}", e)))?;

    let mut paragraphs = Vec::new();

    for child in docx.document.children {
        if let docx_rs::DocumentChild::Paragraph(para) = child {
            let para_text = extract_paragraph_text(&para);
            if !para_text.is_empty() {
                paragraphs.push(para_text);
            }
        } else if let docx_rs::DocumentChild::Table(table) = child {
            let table_text = extract_table_text(&table);
            if !table_text.is_empty() {
                paragraphs.push(table_text);
            }
        }
    }

    if paragraphs.is_empty() {
        return Err(ExtractionError::NoContentFound(
            "Document appears to be empty".into(),
        ));
    }

    Ok(ExtractedContent {
        source: AppSource::Word.as_str().to_string(),
        title: Some(title.to_string()),
        content: paragraphs.join("\n"),
        app_name: "Microsoft Word".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "docx_rs".to_string(),
    })
}

fn extract_paragraph_text(para: &docx_rs::Paragraph) -> String {
    let mut text = String::new();

    for child in &para.children {
        if let docx_rs::ParagraphChild::Run(run) = child {
            for run_child in &run.children {
                if let docx_rs::RunChild::Text(t) = run_child {
                    text.push_str(&t.text);
                }
            }
        }
    }

    text
}

fn extract_table_text(table: &docx_rs::Table) -> String {
    let mut rows = Vec::new();

    for row in &table.rows {
        let docx_rs::TableChild::TableRow(tr) = row;
        let mut cells = Vec::new();
        for cell in &tr.cells {
            let docx_rs::TableRowChild::TableCell(tc) = cell;
            let mut cell_text = String::new();
            for child in &tc.children {
                if let docx_rs::TableCellContent::Paragraph(para) = child {
                    cell_text.push_str(&extract_paragraph_text(para));
                }
            }
            cells.push(cell_text);
        }
        if cells.iter().any(|c| !c.is_empty()) {
            rows.push(cells.join("\t"));
        }
    }

    rows.join("\n")
}


// ============================================================================
// PowerPoint Extraction (custom XML parsing from ZIP)
// ============================================================================

/// Extract content from a PowerPoint file by parsing the XML inside the ZIP
pub fn extract_powerpoint(path: &Path) -> Result<ExtractedContent, ExtractionError> {
    log::info!("[PPTX] Extracting from: {:?}", path);
    
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let temp_path = copy_to_temp(path)?;
    let result = extract_powerpoint_from_path(&temp_path, &file_name);
    let _ = std::fs::remove_file(&temp_path);

    result
}

fn extract_powerpoint_from_path(path: &Path, title: &str) -> Result<ExtractedContent, ExtractionError> {
    let file = std::fs::File::open(path)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to open file: {}", e)))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to read ZIP archive: {}", e)))?;

    // Discover all slide files
    let slide_files = discover_pptx_slides(&mut archive);
    
    if slide_files.is_empty() {
        return Err(ExtractionError::NoContentFound("No slides found in presentation".into()));
    }

    let mut all_slides = Vec::new();

    for (slide_num, slide_path) in slide_files.iter().enumerate() {
        if let Ok(mut slide_file) = archive.by_name(slide_path) {
            let mut xml_content = String::new();
            if slide_file.read_to_string(&mut xml_content).is_ok() {
                let slide_text = extract_text_from_pptx_slide(&xml_content);
                if !slide_text.trim().is_empty() {
                    all_slides.push(format!("=== Slide {} ===\n{}", slide_num + 1, slide_text));
                }
            }
        }
    }

    // Also extract from notes if present
    for slide_num in 1..=slide_files.len() {
        let notes_path = format!("ppt/notesSlides/notesSlide{}.xml", slide_num);
        if let Ok(mut notes_file) = archive.by_name(&notes_path) {
            let mut xml_content = String::new();
            if notes_file.read_to_string(&mut xml_content).is_ok() {
                let notes_text = extract_text_from_pptx_slide(&xml_content);
                if !notes_text.trim().is_empty() {
                    all_slides.push(format!("=== Slide {} Notes ===\n{}", slide_num, notes_text));
                }
            }
        }
    }

    if all_slides.is_empty() {
        return Err(ExtractionError::NoContentFound("Presentation appears to be empty".into()));
    }

    Ok(ExtractedContent {
        source: AppSource::PowerPoint.as_str().to_string(),
        title: Some(title.to_string()),
        content: all_slides.join("\n\n"),
        app_name: "Microsoft PowerPoint".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "pptx_xml".to_string(),
    })
}

/// Discover all slide XML files in a PPTX archive
fn discover_pptx_slides(archive: &mut zip::ZipArchive<std::fs::File>) -> Vec<String> {
    let mut slides = Vec::new();

    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_string();
            // Match ppt/slides/slide1.xml, ppt/slides/slide2.xml, etc.
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") && !name.contains("_rels") {
                slides.push(name);
            }
        }
    }

    // Sort by slide number
    slides.sort_by(|a, b| {
        let num_a = extract_slide_number(a);
        let num_b = extract_slide_number(b);
        num_a.cmp(&num_b)
    });

    slides
}

/// Extract slide number from path like "ppt/slides/slide3.xml"
fn extract_slide_number(path: &str) -> u32 {
    path.trim_start_matches("ppt/slides/slide")
        .trim_start_matches("ppt/notesSlides/notesSlide")
        .trim_end_matches(".xml")
        .parse()
        .unwrap_or(0)
}

/// Extract text from PowerPoint slide XML
/// Text is stored in <a:t> elements within <a:p> (paragraph) elements
fn extract_text_from_pptx_slide(xml: &str) -> String {
    let mut result = Vec::new();
    let mut current_paragraph = String::new();
    let mut in_text_element = false;
    let mut in_paragraph = false;

    let mut chars = xml.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            while let Some(&next) = chars.peek() {
                if next == '>' {
                    chars.next();
                    break;
                }
                tag.push(chars.next().unwrap());
            }

            // Handle paragraph start/end
            if tag.starts_with("a:p") && !tag.starts_with("a:p/") && !tag.contains("/") {
                in_paragraph = true;
                current_paragraph.clear();
            } else if tag == "/a:p" {
                in_paragraph = false;
                let trimmed = current_paragraph.trim();
                if !trimmed.is_empty() {
                    result.push(trimmed.to_string());
                }
                current_paragraph.clear();
            }
            // Handle text element start/end
            else if (tag == "a:t" || tag.starts_with("a:t ")) && !tag.ends_with("/") {
                in_text_element = true;
            } else if tag == "/a:t" {
                in_text_element = false;
            }
        } else if in_text_element && in_paragraph {
            current_paragraph.push(c);
        }
    }

    result.join("\n")
}

// ============================================================================
// Apple iWork Extraction (Pages, Numbers, Keynote)
// ============================================================================

/// Extract content from Apple Pages document
pub fn extract_pages(path: &Path) -> Result<ExtractedContent, ExtractionError> {
    log::info!("[PAGES] Extracting from: {:?}", path);
    
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let temp_path = copy_to_temp(path)?;
    let result = extract_iwork_document(&temp_path, &file_name, AppSource::Pages, "Pages");
    let _ = std::fs::remove_file(&temp_path);

    result
}

/// Extract content from Apple Numbers document
pub fn extract_numbers(path: &Path) -> Result<ExtractedContent, ExtractionError> {
    log::info!("[NUMBERS] Extracting from: {:?}", path);
    
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let temp_path = copy_to_temp(path)?;
    let result = extract_iwork_document(&temp_path, &file_name, AppSource::Numbers, "Numbers");
    let _ = std::fs::remove_file(&temp_path);

    result
}

/// Extract content from Apple Keynote document
pub fn extract_keynote(path: &Path) -> Result<ExtractedContent, ExtractionError> {
    log::info!("[KEYNOTE] Extracting from: {:?}", path);
    
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let temp_path = copy_to_temp(path)?;
    let result = extract_iwork_document(&temp_path, &file_name, AppSource::Keynote, "Keynote");
    let _ = std::fs::remove_file(&temp_path);

    result
}

/// Extract content from an iWork document (Pages, Numbers, or Keynote)
/// 
/// iWork files are ZIP archives containing:
/// - Index/Document.iwa (main document data)
/// - Index/Tables/*.iwa (table data for Numbers)
/// - Various .iwa files with Snappy-compressed Protobuf data
/// 
/// Since the IWA format uses Snappy compression with a non-standard framing
/// and proprietary Protobuf schemas, we use a text extraction approach that
/// searches for readable strings in the decompressed data.
fn extract_iwork_document(
    path: &Path,
    title: &str,
    source: AppSource,
    app_name: &str,
) -> Result<ExtractedContent, ExtractionError> {
    let file = std::fs::File::open(path)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to open file: {}", e)))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to read ZIP archive: {}", e)))?;

    let mut all_text = Vec::new();

    // Find all .iwa files in the archive
    let iwa_files: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            archive.by_index(i).ok().and_then(|f| {
                let name = f.name().to_string();
                if name.ends_with(".iwa") {
                    Some(name)
                } else {
                    None
                }
            })
        })
        .collect();

    log::info!("[IWORK] Found {} .iwa files", iwa_files.len());

    for iwa_path in &iwa_files {
        if let Ok(mut iwa_file) = archive.by_name(iwa_path) {
            let mut compressed_data = Vec::new();
            if iwa_file.read_to_end(&mut compressed_data).is_ok() {
                // Decompress the IWA data (Snappy with custom framing)
                if let Ok(decompressed) = decompress_iwa(&compressed_data) {
                    // Extract readable text from the protobuf data
                    let text = extract_text_from_iwa(&decompressed);
                    if !text.is_empty() {
                        all_text.push(text);
                    }
                }
            }
        }
    }

    if all_text.is_empty() {
        return Err(ExtractionError::NoContentFound(
            format!("{} document appears to be empty", app_name).into(),
        ));
    }

    // Deduplicate and clean up the text
    let content = deduplicate_iwork_text(&all_text);

    Ok(ExtractedContent {
        source: source.as_str().to_string(),
        title: Some(title.to_string()),
        content,
        app_name: app_name.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "iwa_parse".to_string(),
    })
}

/// Decompress IWA data using Snappy with Apple's custom framing
/// 
/// IWA files use Snappy compression but with a non-standard framing format:
/// - No stream identifier chunk
/// - No CRC-32C checksums
/// - Each block starts with: 0x00 followed by 3-byte little-endian length
fn decompress_iwa(data: &[u8]) -> Result<Vec<u8>, ExtractionError> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Each block starts with a header byte (0x00) and 3-byte length
        if pos + 4 > data.len() {
            break;
        }

        let header = data[pos];
        pos += 1;

        // Read 3-byte little-endian length
        let len = data[pos] as usize
            | ((data[pos + 1] as usize) << 8)
            | ((data[pos + 2] as usize) << 16);
        pos += 3;

        if pos + len > data.len() {
            break;
        }

        let chunk = &data[pos..pos + len];
        pos += len;

        match header {
            0x00 => {
                // Snappy compressed chunk
                if let Ok(decompressed) = snap::raw::Decoder::new().decompress_vec(chunk) {
                    result.extend(decompressed);
                }
            }
            0x01 => {
                // Uncompressed chunk
                result.extend_from_slice(chunk);
            }
            _ => {
                // Unknown chunk type, skip
            }
        }
    }

    if result.is_empty() {
        Err(ExtractionError::PlatformError("Failed to decompress IWA data".into()))
    } else {
        Ok(result)
    }
}

/// Extract readable text from decompressed IWA (Protobuf) data
/// 
/// Since we don't have the exact Protobuf schema, we extract text by:
/// 1. Looking for length-prefixed strings (Protobuf wire type 2)
/// 2. Filtering for printable ASCII/UTF-8 text
/// 3. Removing duplicates and noise
fn extract_text_from_iwa(data: &[u8]) -> String {
    let mut texts = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Look for potential string fields (wire type 2 = length-delimited)
        // In protobuf, field tag is (field_number << 3) | wire_type
        // Wire type 2 means the low 3 bits are 010
        
        if pos + 2 > data.len() {
            break;
        }

        // Try to read a varint length
        let (len, bytes_read) = read_varint(&data[pos..]);
        
        if bytes_read == 0 || len == 0 || len > 10000 {
            pos += 1;
            continue;
        }

        let start = pos + bytes_read;
        let end = start + len as usize;

        if end <= data.len() {
            let potential_string = &data[start..end];
            
            // Check if this looks like valid UTF-8 text
            if let Ok(s) = std::str::from_utf8(potential_string) {
                let trimmed = s.trim();
                // Filter: must be mostly printable, reasonable length, not binary garbage
                if is_meaningful_text(trimmed) {
                    texts.push(trimmed.to_string());
                }
            }
        }

        pos += 1;
    }

    texts.join("\n")
}

/// Read a varint from a byte slice
fn read_varint(data: &[u8]) -> (u64, usize) {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut bytes_read = 0;

    for &byte in data.iter().take(10) {
        bytes_read += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        
        if byte & 0x80 == 0 {
            return (result, bytes_read);
        }
        
        shift += 7;
    }

    (0, 0) // Invalid varint
}

/// Check if a string is meaningful text (not binary garbage or metadata)
fn is_meaningful_text(s: &str) -> bool {
    if s.len() < 2 || s.len() > 5000 {
        return false;
    }

    // Count printable characters
    let printable_count = s.chars().filter(|c| {
        c.is_alphanumeric() || c.is_whitespace() || ".,!?;:'\"-()[]{}".contains(*c)
    }).count();

    let total = s.chars().count();
    if total == 0 {
        return false;
    }

    let printable_ratio = printable_count as f64 / total as f64;

    // Must be at least 80% printable characters
    if printable_ratio < 0.8 {
        return false;
    }

    // Must contain at least some letters
    let letter_count = s.chars().filter(|c| c.is_alphabetic()).count();
    if letter_count < 2 {
        return false;
    }

    // Filter out common metadata/internal strings
    let lower = s.to_lowercase();
    let skip_patterns = [
        "com.apple", "iwork", "tswp", "tst.", "tn.", "tsd.", "kn.",
        "uuid", "xmlns", "http://", "https://", ".framework",
        "objc", "class", "selector", "method",
    ];

    for pattern in &skip_patterns {
        if lower.contains(pattern) {
            return false;
        }
    }

    true
}

/// Deduplicate and clean up extracted iWork text
fn deduplicate_iwork_text(texts: &[String]) -> String {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for text in texts {
        for line in text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !seen.contains(trimmed) {
                seen.insert(trimmed.to_string());
                result.push(trimmed.to_string());
            }
        }
    }

    result.join("\n")
}


// ============================================================================
// Main Entry Point
// ============================================================================

/// Extract content from an Office or iWork application by bundle ID.
/// 
/// This function uses a two-tier approach:
/// 1. First, try to get the document path via AXDocument accessibility attribute
///    (preferred - works with sandboxed apps, uses existing Accessibility permission)
/// 2. Fall back to proc_pidinfo to find open files (legacy approach)
pub fn extract_from_office_app(bundle_id: &str) -> Result<ExtractedContent, ExtractionError> {
    log::info!("[FILE-EXTRACT] Extracting from: {}", bundle_id);
    
    // Determine the extraction function based on bundle ID
    let extract_fn: fn(&Path) -> Result<ExtractedContent, ExtractionError> = match bundle_id {
        "com.microsoft.Excel" => extract_excel,
        "com.microsoft.Word" => extract_word,
        "com.microsoft.Powerpoint" => extract_powerpoint,
        "com.apple.iWork.Pages" => extract_pages,
        "com.apple.iWork.Numbers" => extract_numbers,
        "com.apple.iWork.Keynote" => extract_keynote,
        _ => {
            return Err(ExtractionError::PatternNotSupported(format!(
                "Direct file extraction not supported for {}",
                bundle_id
            )))
        }
    };

    // Method 1: Try AXDocument attribute (preferred - works with sandboxed apps)
    log::info!("[FILE-EXTRACT] Trying AXDocument method...");
    match get_document_path_via_ax(bundle_id) {
        Ok(file_path) => {
            log::info!("[FILE-EXTRACT] Got document path via AXDocument: {:?}", file_path);
            
            // Verify the file exists and has a supported extension
            if file_path.exists() {
                // Check if the extension matches what we expect
                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = format!(".{}", ext.to_lowercase());
                    let valid_extensions = match bundle_id {
                        "com.microsoft.Excel" => EXCEL_EXTENSIONS,
                        "com.microsoft.Word" => WORD_EXTENSIONS,
                        "com.microsoft.Powerpoint" => POWERPOINT_EXTENSIONS,
                        "com.apple.iWork.Pages" => PAGES_EXTENSIONS,
                        "com.apple.iWork.Numbers" => NUMBERS_EXTENSIONS,
                        "com.apple.iWork.Keynote" => KEYNOTE_EXTENSIONS,
                        _ => &[],
                    };
                    
                    if valid_extensions.iter().any(|e| *e == ext_lower) {
                        log::info!("[FILE-EXTRACT] Extracting from AXDocument path: {:?}", file_path);
                        return extract_fn(&file_path);
                    } else {
                        log::warn!("[FILE-EXTRACT] AXDocument path has unexpected extension: {}", ext_lower);
                    }
                }
            } else {
                log::warn!("[FILE-EXTRACT] AXDocument path does not exist: {:?}", file_path);
            }
        }
        Err(e) => {
            log::warn!("[FILE-EXTRACT] AXDocument method failed: {}", e);
        }
    }

    // Method 2: Fall back to proc_pidinfo (legacy - may not work with sandboxed apps)
    log::info!("[FILE-EXTRACT] Falling back to proc_pidinfo method...");
    
    let extensions = match bundle_id {
        "com.microsoft.Excel" => EXCEL_EXTENSIONS,
        "com.microsoft.Word" => WORD_EXTENSIONS,
        "com.microsoft.Powerpoint" => POWERPOINT_EXTENSIONS,
        "com.apple.iWork.Pages" => PAGES_EXTENSIONS,
        "com.apple.iWork.Numbers" => NUMBERS_EXTENSIONS,
        "com.apple.iWork.Keynote" => KEYNOTE_EXTENSIONS,
        _ => &[],
    };

    // Find the process
    let pid = get_pid_for_bundle_id(bundle_id).ok_or_else(|| {
        log::error!("[FILE-EXTRACT] Could not find process for {}", bundle_id);
        ExtractionError::AppNotFound(format!("Could not find process for {}", bundle_id))
    })?;

    log::info!("[FILE-EXTRACT] Found PID {} for {}", pid, bundle_id);

    // Find open files using native proc_pidinfo API
    let files = find_open_files(pid, extensions);

    log::info!("[FILE-EXTRACT] Found {} open files: {:?}", files.len(), files);

    if files.is_empty() {
        log::error!("[FILE-EXTRACT] No document files found open");
        return Err(ExtractionError::NoContentFound(
            "No document files found open in the application".into(),
        ));
    }

    // Extract from the first file found
    let file_path = &files[0];
    log::info!("[FILE-EXTRACT] Extracting from file: {:?}", file_path);

    if !file_path.exists() {
        log::error!("[FILE-EXTRACT] File does not exist: {:?}", file_path);
        return Err(ExtractionError::PlatformError(format!(
            "File does not exist: {:?}",
            file_path
        )));
    }

    let result = extract_fn(file_path);
    match &result {
        Ok(content) => {
            log::info!("[FILE-EXTRACT] Extraction successful: {} chars", content.content.len());
        }
        Err(e) => {
            log::error!("[FILE-EXTRACT] Extraction failed: {}", e);
        }
    }
    
    result
}

/// Check if direct file extraction is supported for a bundle ID
pub fn supports_direct_extraction(bundle_id: &str) -> bool {
    matches!(
        bundle_id,
        "com.microsoft.Excel"
            | "com.microsoft.Word"
            | "com.microsoft.Powerpoint"
            | "com.apple.iWork.Pages"
            | "com.apple.iWork.Numbers"
            | "com.apple.iWork.Keynote"
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_direct_extraction() {
        // Microsoft Office
        assert!(supports_direct_extraction("com.microsoft.Excel"));
        assert!(supports_direct_extraction("com.microsoft.Word"));
        assert!(supports_direct_extraction("com.microsoft.Powerpoint"));
        
        // Apple iWork
        assert!(supports_direct_extraction("com.apple.iWork.Pages"));
        assert!(supports_direct_extraction("com.apple.iWork.Numbers"));
        assert!(supports_direct_extraction("com.apple.iWork.Keynote"));
        
        // Not supported
        assert!(!supports_direct_extraction("com.apple.Safari"));
        assert!(!supports_direct_extraction("com.google.Chrome"));
    }

    #[test]
    fn test_excel_extensions() {
        assert!(EXCEL_EXTENSIONS.contains(&".xlsx"));
        assert!(EXCEL_EXTENSIONS.contains(&".xls"));
        assert!(EXCEL_EXTENSIONS.contains(&".xlsb"));
    }

    #[test]
    fn test_powerpoint_extensions() {
        assert!(POWERPOINT_EXTENSIONS.contains(&".pptx"));
    }

    #[test]
    fn test_iwork_extensions() {
        assert!(PAGES_EXTENSIONS.contains(&".pages"));
        assert!(NUMBERS_EXTENSIONS.contains(&".numbers"));
        assert!(KEYNOTE_EXTENSIONS.contains(&".key"));
    }

    #[test]
    fn test_extract_slide_number() {
        assert_eq!(extract_slide_number("ppt/slides/slide1.xml"), 1);
        assert_eq!(extract_slide_number("ppt/slides/slide10.xml"), 10);
        assert_eq!(extract_slide_number("ppt/slides/slide123.xml"), 123);
    }

    #[test]
    fn test_is_meaningful_text() {
        // Valid text
        assert!(is_meaningful_text("Hello, world!"));
        assert!(is_meaningful_text("This is a test document."));
        
        // Too short
        assert!(!is_meaningful_text("a"));
        
        // Contains skip patterns
        assert!(!is_meaningful_text("com.apple.iWork.Pages"));
        assert!(!is_meaningful_text("TSWP.StorageArchive"));
        
        // Binary garbage (non-printable)
        assert!(!is_meaningful_text("\x00\x01\x02\x03"));
    }

    #[test]
    fn test_read_varint() {
        // Single byte varint
        assert_eq!(read_varint(&[0x01]), (1, 1));
        assert_eq!(read_varint(&[0x7F]), (127, 1));
        
        // Two byte varint
        assert_eq!(read_varint(&[0x80, 0x01]), (128, 2));
        assert_eq!(read_varint(&[0xAC, 0x02]), (300, 2));
    }

    #[test]
    fn test_extract_text_from_pptx_slide() {
        let xml = r#"
            <p:sp>
                <p:txBody>
                    <a:p>
                        <a:r><a:t>Hello</a:t></a:r>
                        <a:r><a:t> World</a:t></a:r>
                    </a:p>
                    <a:p>
                        <a:r><a:t>Second paragraph</a:t></a:r>
                    </a:p>
                </p:txBody>
            </p:sp>
        "#;
        
        let text = extract_text_from_pptx_slide(xml);
        assert!(text.contains("Hello World"));
        assert!(text.contains("Second paragraph"));
    }
}
