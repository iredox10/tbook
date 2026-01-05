use anyhow::{Context, Result};
use std::path::Path;

pub struct PdfParser {
    path: String,
    pages: Vec<String>,
}

impl PdfParser {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Use lopdf directly to get page count and extract text per page for better navigation
        let doc = lopdf::Document::load(&path).context("Failed to load PDF document")?;
        let page_numbers = doc.get_pages();
        let mut pages = Vec::new();

        for (page_num, _) in page_numbers {
            if let Ok(text) = doc.extract_text(&[page_num]) {
                if !text.trim().is_empty() {
                    pages.push(text);
                }
            }
        }

        if pages.is_empty() {
            // Fallback to extract all if per-page failed or returned empty
            let content =
                pdf_extract::extract_text(&path).context("Failed to extract text from PDF")?;
            pages.push(content);
        }

        Ok(Self {
            path: path_str,
            pages,
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
        self.pages.len()
    }

    pub fn get_chapter_content(&mut self, index: usize) -> Result<String> {
        self.pages
            .get(index)
            .cloned()
            .context("Page index out of bounds")
    }

    pub fn get_toc(&self) -> Vec<String> {
        (0..self.pages.len())
            .map(|i| format!("Page {}", i + 1))
            .collect()
    }
}
