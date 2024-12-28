use anyhow::{Context, Result};
use rig::{
    loaders::PdfFileLoader,
    Embed,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use std::path::PathBuf;
use std::fs;

#[derive(Embed, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
struct Document {
    id: String,
    #[embed]
    content: String,
}

pub fn load_dialog_pdf(path: PathBuf) -> Result<Vec<String>> {
    let mut chunks = Vec::new();

    for entry in PdfFileLoader::with_glob(path.to_str().unwrap())?.read() {
        let content = entry?;
        println!("Raw PDF Content: {:?}", content);
        // Split content by line breaks
        for line in content.lines() {
            let trimmed_line = line.trim();
            // Only include non-empty lines as chunks
            if !trimmed_line.is_empty() {
                chunks.push(trimmed_line.to_string());
            }
        }
    }
    if chunks.is_empty() {
        anyhow::bail!("No content found in PDF file: {:?}", path);
    }

    Ok(chunks)
}

pub fn load_pdf_flattened(path: PathBuf) -> Result<Vec<String>> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let chunk_size = 2000; // Approximately 2000 characters per chunk

    for entry in PdfFileLoader::with_glob(path.to_str().unwrap())?.read() {
        let content = entry?;

        // Split content into words
        let words: Vec<&str> = content.split_whitespace().collect();

        for word in words {
            if current_chunk.len() + word.len() + 1 > chunk_size {
                // If adding the next word would exceed chunk size,
                // save current chunk and start a new one
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk.clear();
                }
            }
            current_chunk.push_str(word);
            current_chunk.push(' ');
        }
    }

    // Don't forget the last chunk
    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    if chunks.is_empty() {
        anyhow::bail!("No content found in PDF file: {:?}", path);
    }

    Ok(chunks)
}

pub fn load_pdfs_from_dir(documents_dir: PathBuf) -> Result<Vec<(String, Vec<String>)>> {
    let mut pdf_chunks = Vec::new();

    for entry in fs::read_dir(&documents_dir).context("Failed to read documents directory")? {
        let entry = entry.context("Failed to read entry")?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "pdf") {
            let file_name = path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Unknown file")
                .to_string();

            let chunks = load_dialog_pdf(path).with_context(|| format!("Failed to load {}", file_name))?;

            pdf_chunks.push((file_name, chunks));
        }
    }

    Ok(pdf_chunks)
}
