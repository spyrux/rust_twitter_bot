use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Load lines from a single `.txt` file, considering only Gi-hun's dialogue and separating by blank lines.
pub fn load_txt_lines(path: &Path) -> Result<Vec<String>, io::Error> {
    let file = File::open(path)?;
    let lines = io::BufReader::new(file)
        .lines()
        .filter_map(|line| line.ok()) // Filter out invalid lines gracefully
        .collect::<Vec<String>>();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for line in lines {
        let trimmed_line = line.trim();

        // If the line is empty, treat this as a separator (new chunk)
        if trimmed_line.is_empty() {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk.clone());
                current_chunk.clear();
            }
            continue;
        }

        // Ensure that we are collecting only Gi-hun's lines
        if let Some(speaker) = line.split(':').next() {
            if speaker.trim() == "Gi-hun" {
                current_chunk.push_str(trimmed_line);  // Add dialogue text to current chunk
                current_chunk.push_str("\n");  // To retain the format, add newline
            }
        }
    }

    // If there’s any remaining chunk, push it (in case the last chunk is not followed by a blank line)
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    Ok(chunks)
}

/// Load all `.txt` files from a directory and return Gi-hun's dialogue from each as separate chunks.
pub fn load_txts_from_dir(documents_dir: PathBuf) -> Result<Vec<(String, Vec<String>)>> {
    let mut txt_chunks = Vec::new();

    for entry in fs::read_dir(&documents_dir).context("Failed to read documents directory")? {
        let entry = entry.context("Failed to read entry")?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "txt") { // Handle `.txt` files
            let file_name = path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Unknown file")
                .to_string();

            let chunks = load_txt_lines(&path)
                .with_context(|| format!("Failed to load {}", file_name))?;

            txt_chunks.push((file_name, chunks)); // Append filename and chunks
        }
    }

    Ok(txt_chunks)
}