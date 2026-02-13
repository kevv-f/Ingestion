//! AppleScript integration for extracting content from applications.
//!
//! Microsoft Office apps don't fully expose document content through the
//! Accessibility API, but we can read .docx/.xlsx files directly since they
//! are ZIP archives containing XML. For unsaved documents, we fall back to
//! AppleScript to get the file path.

use std::process::Command;
use std::path::Path;
use std::io::Read;
use crate::types::{AppSource, ExtractedContent, ExtractionError};

/// Extract content from Microsoft Word.
///
/// This function first tries to get the file path of the active document,
/// then reads the .docx file directly to extract text content.
pub fn extract_from_word() -> Result<ExtractedContent, ExtractionError> {
    // Get the file path of the active document
    let path_script = r#"
        tell application "Microsoft Word"
            if (count of documents) > 0 then
                set docPath to full name of active document
                set docName to name of active document
                return docPath & "|||" & docName
            else
                error "No document open"
            end if
        end tell
    "#;
    
    let output = Command::new("osascript")
        .arg("-e")
        .arg(path_script)
        .output()
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to run osascript: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ExtractionError::PlatformError(format!(
            "AppleScript failed: {}", stderr.trim()
        )));
    }
    
    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parts: Vec<&str> = result.splitn(2, "|||").collect();
    
    if parts.len() != 2 {
        return Err(ExtractionError::PlatformError("Failed to get document path".into()));
    }
    
    let file_path = parts[0];
    let doc_name = parts[1].to_string();
    
    // Try to read the .docx file directly
    if file_path.ends_with(".docx") && Path::new(file_path).exists() {
        match extract_text_from_docx(file_path) {
            Ok(content) => {
                return Ok(ExtractedContent {
                    source: AppSource::Word.as_str().to_string(),
                    title: Some(doc_name),
                    content,
                    app_name: "Microsoft Word".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                    extraction_method: "docx_parse".to_string(),
                });
            }
            Err(e) => {
                log::warn!("Failed to parse .docx file directly: {}", e);
                // Fall through to AppleScript fallback
            }
        }
    }
    
    // Fallback: try to get content via AppleScript (less reliable)
    let content_script = r#"
        tell application "Microsoft Word"
            set docContent to content of text object of active document
            return docContent
        end tell
    "#;
    
    let content_output = Command::new("osascript")
        .arg("-e")
        .arg(content_script)
        .output()
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to run osascript: {}", e)))?;
    
    let content = if content_output.status.success() {
        String::from_utf8_lossy(&content_output.stdout).trim().to_string()
    } else {
        return Err(ExtractionError::NoContentFound(
            "Could not extract content from Word document. The document may be unsaved.".into()
        ));
    };
    
    if content.is_empty() {
        return Err(ExtractionError::NoContentFound("Document appears to be empty".into()));
    }
    
    Ok(ExtractedContent {
        source: AppSource::Word.as_str().to_string(),
        title: Some(doc_name),
        content,
        app_name: "Microsoft Word".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "applescript".to_string(),
    })
}

/// Extract text content from a .docx file by parsing its XML.
///
/// .docx files are ZIP archives containing XML files. The main document
/// content is in word/document.xml.
fn extract_text_from_docx(path: &str) -> Result<String, ExtractionError> {
    let file = std::fs::File::open(path)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to open file: {}", e)))?;
    
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to read ZIP archive: {}", e)))?;
    
    // Read word/document.xml
    let mut document_xml = archive.by_name("word/document.xml")
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to find document.xml: {}", e)))?;
    
    let mut xml_content = String::new();
    document_xml.read_to_string(&mut xml_content)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to read document.xml: {}", e)))?;
    
    // Parse XML and extract text from <w:t> elements
    let text = extract_text_from_word_xml(&xml_content);
    
    Ok(text)
}

/// Extract text from Word XML content.
///
/// Word documents store text in <w:t> elements within <w:p> (paragraph) elements.
fn extract_text_from_word_xml(xml: &str) -> String {
    let mut result = String::new();
    let mut in_text_element = false;
    let mut current_text = String::new();
    let mut last_was_paragraph = false;
    
    // Simple XML parsing - look for <w:t> and </w:t> tags
    let mut chars = xml.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '<' {
            // Check what tag this is
            let mut tag = String::new();
            while let Some(&next) = chars.peek() {
                if next == '>' {
                    chars.next();
                    break;
                }
                tag.push(chars.next().unwrap());
            }
            
            if tag.starts_with("w:t") && !tag.starts_with("w:t/") {
                // Opening <w:t> tag (might have attributes like <w:t xml:space="preserve">)
                in_text_element = true;
                current_text.clear();
            } else if tag == "/w:t" {
                // Closing </w:t> tag
                in_text_element = false;
                result.push_str(&current_text);
                last_was_paragraph = false;
            } else if tag == "/w:p" || tag == "w:p/" {
                // End of paragraph - add newline
                if !last_was_paragraph && !result.is_empty() {
                    result.push('\n');
                    last_was_paragraph = true;
                }
            }
        } else if in_text_element {
            current_text.push(c);
        }
    }
    
    // Normalize line endings
    result.replace('\r', "\n").trim().to_string()
}

/// Extract content from Microsoft Excel.
///
/// This function first tries to get the file path of the active workbook,
/// then reads the .xlsx file directly to extract text content.
pub fn extract_from_excel() -> Result<ExtractedContent, ExtractionError> {
    // Get the file path of the active workbook
    let path_script = r#"
        tell application "Microsoft Excel"
            if (count of workbooks) > 0 then
                set wbPath to full name of active workbook
                set wbName to name of active workbook
                return wbPath & "|||" & wbName
            else
                error "No workbook open"
            end if
        end tell
    "#;
    
    let output = Command::new("osascript")
        .arg("-e")
        .arg(path_script)
        .output()
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to run osascript: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ExtractionError::PlatformError(format!(
            "AppleScript failed: {}", stderr.trim()
        )));
    }
    
    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parts: Vec<&str> = result.splitn(2, "|||").collect();
    
    if parts.len() != 2 {
        return Err(ExtractionError::PlatformError("Failed to get workbook path".into()));
    }
    
    let file_path = parts[0];
    let wb_name = parts[1].to_string();
    
    // Try to read the .xlsx file directly
    if file_path.ends_with(".xlsx") && Path::new(file_path).exists() {
        match extract_text_from_xlsx(file_path) {
            Ok(content) => {
                return Ok(ExtractedContent {
                    source: AppSource::Excel.as_str().to_string(),
                    title: Some(wb_name),
                    content,
                    app_name: "Microsoft Excel".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                    extraction_method: "xlsx_parse".to_string(),
                });
            }
            Err(e) => {
                log::warn!("Failed to parse .xlsx file directly: {}", e);
                // Fall through to AppleScript fallback
            }
        }
    }
    
    // Fallback: try to get content via AppleScript (extracts ALL sheets)
    let content_script = r#"
        tell application "Microsoft Excel"
            set textContent to ""
            
            repeat with ws in worksheets of active workbook
                set sheetName to name of ws
                set textContent to textContent & "=== " & sheetName & " ===" & linefeed
                
                try
                    set usedRange to used range of ws
                    set cellValues to value of cells of usedRange
                    
                    repeat with rowData in cellValues
                        repeat with cellValue in rowData
                            if cellValue is not missing value then
                                set textContent to textContent & (cellValue as text) & tab
                            end if
                        end repeat
                        set textContent to textContent & linefeed
                    end repeat
                end try
                
                set textContent to textContent & linefeed
            end repeat
            
            return textContent
        end tell
    "#;
    
    let content_output = Command::new("osascript")
        .arg("-e")
        .arg(content_script)
        .output()
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to run osascript: {}", e)))?;
    
    let content = if content_output.status.success() {
        String::from_utf8_lossy(&content_output.stdout).trim().to_string()
    } else {
        return Err(ExtractionError::NoContentFound(
            "Could not extract content from Excel workbook. The workbook may be unsaved.".into()
        ));
    };
    
    if content.is_empty() {
        return Err(ExtractionError::NoContentFound("Workbook appears to be empty".into()));
    }
    
    Ok(ExtractedContent {
        source: AppSource::Excel.as_str().to_string(),
        title: Some(wb_name),
        content,
        app_name: "Microsoft Excel".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "applescript".to_string(),
    })
}

/// Extract text content from an .xlsx file by parsing its XML.
///
/// .xlsx files are ZIP archives containing XML files. The cell values are
/// stored in xl/sharedStrings.xml and xl/worksheets/sheet{N}.xml.
/// This function extracts content from ALL worksheets in the workbook.
fn extract_text_from_xlsx(path: &str) -> Result<String, ExtractionError> {
    log::info!("[XLSX-DEBUG] Opening xlsx file: {}", path);
    
    let file = std::fs::File::open(path)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to open file: {}", e)))?;
    
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to read ZIP archive: {}", e)))?;
    
    log::info!("[XLSX-DEBUG] Archive contains {} files", archive.len());
    
    // First, read shared strings (Excel stores text in a shared string table)
    let shared_strings = if let Ok(mut ss_file) = archive.by_name("xl/sharedStrings.xml") {
        let mut xml_content = String::new();
        ss_file.read_to_string(&mut xml_content)
            .map_err(|e| ExtractionError::PlatformError(format!("Failed to read sharedStrings.xml: {}", e)))?;
        let strings = parse_shared_strings(&xml_content);
        log::info!("[XLSX-DEBUG] Loaded {} shared strings", strings.len());
        strings
    } else {
        log::info!("[XLSX-DEBUG] No sharedStrings.xml found (workbook may have no text)");
        Vec::new()
    };
    
    // Get worksheet names from workbook.xml
    let sheet_names = get_worksheet_names(&mut archive)?;
    log::info!("[XLSX-DEBUG] Sheet names from workbook.xml: {:?}", sheet_names);
    
    // Discover all worksheet files in the archive
    let worksheet_files = discover_worksheet_files(&mut archive);
    log::info!("[XLSX-DEBUG] Discovered worksheet files: {:?}", worksheet_files);
    
    if worksheet_files.is_empty() {
        log::error!("[XLSX-DEBUG] No worksheet files found in archive!");
        return Err(ExtractionError::PlatformError("No worksheets found".into()));
    }
    
    // Extract content from all worksheets
    let mut all_content = Vec::new();
    
    for (idx, sheet_path) in worksheet_files.iter().enumerate() {
        log::info!("[XLSX-DEBUG] Processing worksheet {}/{}: {}", idx + 1, worksheet_files.len(), sheet_path);
        
        let mut sheet_xml = String::new();
        match archive.by_name(sheet_path) {
            Ok(mut sheet_file) => {
                match sheet_file.read_to_string(&mut sheet_xml) {
                    Ok(bytes_read) => {
                        log::info!("[XLSX-DEBUG] Read {} bytes from {}", bytes_read, sheet_path);
                        let sheet_content = extract_text_from_excel_sheet(&sheet_xml, &shared_strings);
                        let content_len = sheet_content.len();
                        let trimmed_len = sheet_content.trim().len();
                        
                        log::info!("[XLSX-DEBUG] Sheet {} content: {} chars ({} trimmed)", 
                            sheet_path, content_len, trimmed_len);
                        
                        if !sheet_content.trim().is_empty() {
                            // Get sheet name if available, otherwise use index
                            let sheet_name = sheet_names.get(idx)
                                .cloned()
                                .unwrap_or_else(|| format!("Sheet {}", idx + 1));
                            log::info!("[XLSX-DEBUG] Adding sheet '{}' with {} chars", sheet_name, trimmed_len);
                            all_content.push(format!("=== {} ===\n{}", sheet_name, sheet_content));
                        } else {
                            log::info!("[XLSX-DEBUG] Sheet {} is empty, skipping", sheet_path);
                        }
                    }
                    Err(e) => {
                        log::error!("[XLSX-DEBUG] Failed to read {}: {}", sheet_path, e);
                    }
                }
            }
            Err(e) => {
                log::error!("[XLSX-DEBUG] Failed to open {}: {}", sheet_path, e);
            }
        }
    }
    
    log::info!("[XLSX-DEBUG] Extraction complete: {} sheets with content out of {} total", 
        all_content.len(), worksheet_files.len());
    
    if all_content.is_empty() {
        return Err(ExtractionError::NoContentFound("All worksheets appear to be empty".into()));
    }
    
    Ok(all_content.join("\n\n"))
}

/// Get worksheet names from xl/workbook.xml
fn get_worksheet_names(archive: &mut zip::ZipArchive<std::fs::File>) -> Result<Vec<String>, ExtractionError> {
    let mut names = Vec::new();
    
    if let Ok(mut workbook_file) = archive.by_name("xl/workbook.xml") {
        let mut xml_content = String::new();
        if workbook_file.read_to_string(&mut xml_content).is_ok() {
            names = parse_sheet_names_from_workbook(&xml_content);
        }
    }
    
    Ok(names)
}

/// Parse sheet names from workbook.xml
/// Looks for <sheet name="SheetName" .../> elements
fn parse_sheet_names_from_workbook(xml: &str) -> Vec<String> {
    let mut names = Vec::new();
    
    // Find all <sheet ... name="..." .../> elements
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
            
            // Check if this is a sheet element
            if tag.starts_with("sheet ") || tag.starts_with("sheet\t") {
                // Extract the name attribute
                if let Some(name) = extract_xml_attribute(&tag, "name") {
                    names.push(name);
                }
            }
        }
    }
    
    names
}

/// Extract an attribute value from an XML tag string
fn extract_xml_attribute(tag: &str, attr_name: &str) -> Option<String> {
    // Look for name="value" or name='value'
    let patterns = [
        format!("{}=\"", attr_name),
        format!("{}='", attr_name),
    ];
    
    for pattern in &patterns {
        if let Some(start_idx) = tag.find(pattern) {
            let value_start = start_idx + pattern.len();
            let quote_char = if pattern.ends_with('"') { '"' } else { '\'' };
            
            if let Some(end_idx) = tag[value_start..].find(quote_char) {
                return Some(tag[value_start..value_start + end_idx].to_string());
            }
        }
    }
    
    None
}

/// Discover all worksheet files in the xlsx archive
fn discover_worksheet_files(archive: &mut zip::ZipArchive<std::fs::File>) -> Vec<String> {
    let mut worksheets = Vec::new();
    
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_string();
            if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
                worksheets.push(name);
            }
        }
    }
    
    // Sort by sheet number to maintain order (sheet1.xml, sheet2.xml, etc.)
    worksheets.sort_by(|a, b| {
        let num_a = extract_sheet_number(a);
        let num_b = extract_sheet_number(b);
        num_a.cmp(&num_b)
    });
    
    worksheets
}

/// Extract the sheet number from a worksheet filename like "xl/worksheets/sheet3.xml"
fn extract_sheet_number(path: &str) -> u32 {
    path.trim_start_matches("xl/worksheets/sheet")
        .trim_end_matches(".xml")
        .parse()
        .unwrap_or(0)
}

/// Parse shared strings from Excel's sharedStrings.xml
fn parse_shared_strings(xml: &str) -> Vec<String> {
    let mut strings = Vec::new();
    let mut in_text_element = false;
    let mut current_text = String::new();
    
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
            
            if tag == "t" || tag.starts_with("t ") {
                in_text_element = true;
                current_text.clear();
            } else if tag == "/t" {
                in_text_element = false;
                strings.push(current_text.clone());
            } else if tag == "/si" {
                // End of string item - if we haven't captured text yet, add empty
                if strings.is_empty() || !current_text.is_empty() {
                    // Already added in /t handler
                }
            }
        } else if in_text_element {
            current_text.push(c);
        }
    }
    
    strings
}

/// Extract text from Excel worksheet XML
fn extract_text_from_excel_sheet(xml: &str, shared_strings: &[String]) -> String {
    let mut result = String::new();
    let mut current_row = 0u32;
    let mut in_value = false;
    let mut current_value = String::new();
    let mut is_shared_string = false;
    
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
            
            if tag.starts_with("row ") {
                // New row - add newline if not first row
                if current_row > 0 {
                    result.push('\n');
                }
                current_row += 1;
            } else if tag.starts_with("c ") {
                // Cell - check if it's a shared string type
                is_shared_string = tag.contains("t=\"s\"");
            } else if tag == "v" {
                in_value = true;
                current_value.clear();
            } else if tag == "/v" {
                in_value = false;
                
                // Get the actual value
                let value = if is_shared_string {
                    // Look up in shared strings table
                    if let Ok(idx) = current_value.parse::<usize>() {
                        shared_strings.get(idx).cloned().unwrap_or_default()
                    } else {
                        current_value.clone()
                    }
                } else {
                    current_value.clone()
                };
                
                if !value.is_empty() {
                    if !result.is_empty() && !result.ends_with('\n') {
                        result.push('\t');
                    }
                    result.push_str(&value);
                }
            }
        } else if in_value {
            current_value.push(c);
        }
    }
    
    result.trim().to_string()
}

/// Extract content from Microsoft PowerPoint using AppleScript.
pub fn extract_from_powerpoint() -> Result<ExtractedContent, ExtractionError> {
    let script = r#"
        tell application "Microsoft PowerPoint"
            if (count of presentations) > 0 then
                set presName to name of active presentation
                set textContent to ""
                
                repeat with slideItem in slides of active presentation
                    repeat with shapeItem in shapes of slideItem
                        if has text frame of shapeItem then
                            set tf to text frame of shapeItem
                            if has text of tf then
                                set textContent to textContent & (content of text range of tf) & linefeed
                            end if
                        end if
                    end repeat
                    set textContent to textContent & linefeed
                end repeat
                
                return presName & "|||" & textContent
            else
                error "No presentation open"
            end if
        end tell
    "#;
    
    execute_applescript_extraction(script, AppSource::PowerPoint, "com.microsoft.Powerpoint")
}

/// Extract content from Apple Pages using AppleScript.
pub fn extract_from_pages() -> Result<ExtractedContent, ExtractionError> {
    let script = r#"
        tell application "Pages"
            if (count of documents) > 0 then
                set docName to name of document 1
                set docContent to body text of document 1
                return docName & "|||" & docContent
            else
                error "No document open"
            end if
        end tell
    "#;
    
    execute_applescript_extraction(script, AppSource::Pages, "com.apple.iWork.Pages")
}

/// Extract content from Apple Numbers using AppleScript.
pub fn extract_from_numbers() -> Result<ExtractedContent, ExtractionError> {
    let script = r#"
        tell application "Numbers"
            if (count of documents) > 0 then
                set docName to name of document 1
                set textContent to ""
                
                tell document 1
                    tell active sheet
                        repeat with t in tables
                            set rowCount to row count of t
                            set colCount to column count of t
                            repeat with r from 1 to rowCount
                                repeat with c from 1 to colCount
                                    set cellValue to value of cell r of column c of t
                                    if cellValue is not missing value then
                                        set textContent to textContent & (cellValue as text) & tab
                                    end if
                                end repeat
                                set textContent to textContent & linefeed
                            end repeat
                        end repeat
                    end tell
                end tell
                
                return docName & "|||" & textContent
            else
                error "No document open"
            end if
        end tell
    "#;
    
    execute_applescript_extraction(script, AppSource::Numbers, "com.apple.iWork.Numbers")
}

/// Extract content from Apple Keynote using AppleScript.
pub fn extract_from_keynote() -> Result<ExtractedContent, ExtractionError> {
    let script = r#"
        tell application "Keynote"
            if (count of documents) > 0 then
                set docName to name of document 1
                set textContent to ""
                
                tell document 1
                    repeat with slideItem in slides
                        repeat with textItem in text items of slideItem
                            set textContent to textContent & (object text of textItem) & linefeed
                        end repeat
                        set textContent to textContent & linefeed
                    end repeat
                end tell
                
                return docName & "|||" & textContent
            else
                error "No document open"
            end if
        end tell
    "#;
    
    execute_applescript_extraction(script, AppSource::Keynote, "com.apple.iWork.Keynote")
}

/// Execute an AppleScript and parse the result into ExtractedContent.
fn execute_applescript_extraction(
    script: &str,
    source: AppSource,
    app_name: &str,
) -> Result<ExtractedContent, ExtractionError> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| ExtractionError::PlatformError(format!("Failed to run osascript: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ExtractionError::PlatformError(format!(
            "AppleScript failed: {}", stderr.trim()
        )));
    }
    
    let result = String::from_utf8_lossy(&output.stdout);
    let result = result.trim();
    
    // Parse the result (format: "docName|||content")
    let parts: Vec<&str> = result.splitn(2, "|||").collect();
    
    let (title, content) = if parts.len() == 2 {
        (Some(parts[0].to_string()), parts[1].to_string())
    } else {
        (None, result.to_string())
    };
    
    if content.trim().is_empty() {
        return Err(ExtractionError::NoContentFound("Document appears to be empty".into()));
    }
    
    Ok(ExtractedContent {
        source: source.as_str().to_string(),
        title,
        content,
        app_name: app_name.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        extraction_method: "applescript".to_string(),
    })
}

/// Check if an app supports AppleScript extraction.
pub fn supports_applescript(bundle_id: &str) -> bool {
    matches!(bundle_id,
        "com.microsoft.Word" |
        "com.microsoft.Excel" |
        "com.microsoft.Powerpoint" |
        "com.apple.iWork.Pages" |
        "com.apple.iWork.Numbers" |
        "com.apple.iWork.Keynote"
    )
}

/// Extract content using AppleScript based on bundle ID.
pub fn extract_via_applescript(bundle_id: &str) -> Result<ExtractedContent, ExtractionError> {
    match bundle_id {
        "com.microsoft.Word" => extract_from_word(),
        "com.microsoft.Excel" => extract_from_excel(),
        "com.microsoft.Powerpoint" => extract_from_powerpoint(),
        "com.apple.iWork.Pages" => extract_from_pages(),
        "com.apple.iWork.Numbers" => extract_from_numbers(),
        "com.apple.iWork.Keynote" => extract_from_keynote(),
        _ => Err(ExtractionError::PatternNotSupported(format!(
            "AppleScript extraction not supported for {}", bundle_id
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_supports_applescript() {
        assert!(supports_applescript("com.microsoft.Word"));
        assert!(supports_applescript("com.microsoft.Excel"));
        assert!(supports_applescript("com.apple.iWork.Pages"));
        assert!(!supports_applescript("com.apple.Safari"));
        assert!(!supports_applescript("com.google.Chrome"));
    }
}
