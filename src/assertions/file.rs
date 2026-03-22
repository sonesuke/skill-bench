// File assertions
// Ported from check-file-content.sh, check-file-not-contains.sh,
// check-workspace-file.sh, check-workspace-dir.sh, check-output-file.sh

use std::path::Path;

/// Check if workspace file contains specific content
/// contains: true means file should contain the string, false means it should NOT
pub fn check_file_content(
    work_dir: &Path,
    filename: &str,
    search_string: &str,
    contains: bool,
) -> Result<(), String> {
    let file_path = work_dir.join(filename);

    if !file_path.exists() {
        return Err(format!("File '{}' does not exist in workspace", filename));
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file '{}': {}", filename, e))?;

    let found = content.contains(search_string);

    if contains {
        if found {
            Ok(())
        } else {
            Err(format!(
                "File '{}' does not contain expected string '{}'",
                filename, search_string
            ))
        }
    } else if !found {
        Ok(())
    } else {
        Err(format!(
            "File '{}' contains unexpected string '{}'",
            filename, search_string
        ))
    }
}

/// Check if workspace file exists
pub fn check_workspace_file(work_dir: &Path, file_path: &str) -> Result<(), String> {
    let full_path = work_dir.join(file_path);

    if full_path.exists() && full_path.is_file() {
        Ok(())
    } else {
        Err(format!("Workspace file '{}' does not exist", file_path))
    }
}

/// Check if workspace directory exists
pub fn check_workspace_dir(work_dir: &Path, dir_paths: &[&str]) -> Result<(), String> {
    for dir_path in dir_paths {
        let full_path = work_dir.join(dir_path);

        if !full_path.exists() || !full_path.is_dir() {
            return Err(format!("Workspace directory '{}' does not exist", dir_path));
        }
    }

    Ok(())
}

/// Check if output file was created
///
/// # Arguments
/// * `work_dir` - Working directory
/// * `filename` - Specific filename to check (empty string = check common patterns)
#[allow(dead_code)]
pub fn check_output_file(work_dir: &Path, filename: &str) -> Result<(), String> {
    // If specific filename provided, check only that
    if !filename.is_empty() && filename != "{}" {
        let full_path = work_dir.join(filename);
        if full_path.exists() {
            return Ok(());
        }
        return Err(format!("Output file '{}' not found", filename));
    }

    // Check for common output files (backward compatible)
    let output_files = [
        "output.txt",
        "output.md",
        "result.txt",
        "result.md",
        "PROGRESS.md",
    ];

    for file in output_files {
        let full_path = work_dir.join(file);
        if full_path.exists() {
            return Ok(());
        }
    }

    Err("No output file found".to_string())
}
