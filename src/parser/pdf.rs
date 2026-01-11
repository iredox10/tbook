use anyhow::{Context, Result};
use pdf::file::FileOptions;
use std::path::Path;
use std::process::Command;

pub struct PdfParser {
    path: String,
    page_count: usize,
}

impl PdfParser {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Use pdf crate to open the file lazily and get page count
        // This avoids loading the whole document into memory
        let file = FileOptions::cached()
            .open(&path_str)
            .context("Failed to open PDF document")?;

        let page_count = file.num_pages() as usize;

        Ok(Self {
            path: path_str,
            page_count,
        })
    }

    pub fn get_metadata(&self) -> (String, String) {
        let title = Path::new(&self.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown PDF")
            .to_string();
        (title, "PDF Document".to_string())
    }

    pub fn get_chapter_count(&self) -> usize {
        self.page_count
    }

    pub fn get_chapter_content(&mut self, index: usize) -> Result<String> {
        // Use pdftotext CLI for robust and fast text extraction of a single page
        // Pages are 1-based in pdftotext
        let page_num = index + 1;

        let output = Command::new("pdftotext")
            .args(&[
                "-f",
                &page_num.to_string(),
                "-l",
                &page_num.to_string(),
                "-layout", // Preserve layout
                &self.path,
                "-", // Output to stdout
            ])
            .output()
            .context("Failed to execute pdftotext. Ensure poppler-utils is installed.")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("pdftotext failed: {}", stderr));
        }

        let text = String::from_utf8_lossy(&output.stdout).to_string();

        if text.trim().is_empty() {
            Ok(" [ Blank Page or Text Not Extractable ] ".to_string())
        } else {
            Ok(text)
        }
    }

    pub fn get_toc(&self) -> Vec<String> {
        (0..self.page_count)
            .map(|i| format!("Page {}", i + 1))
            .collect()
    }
}
