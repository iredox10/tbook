use anyhow::{Context, Result};
use epub::doc::EpubDoc;
use html2text::from_read;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct EpubParser {
    doc: EpubDoc<BufReader<File>>,
}

impl EpubParser {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let doc = EpubDoc::new(path).context("Failed to open EPUB document")?;
        Ok(Self { doc })
    }

    pub fn get_metadata(&self) -> (String, String) {
        let title = self
            .doc
            .mdata("title")
            .map(|v| v.value.clone())
            .unwrap_or_else(|| "Unknown Title".to_string());
        let author = self
            .doc
            .mdata("creator")
            .map(|v| v.value.clone())
            .unwrap_or_else(|| "Unknown Author".to_string());
        (title, author)
    }

    pub fn get_chapter_count(&self) -> usize {
        self.doc.spine.len()
    }

    pub fn get_chapter_content(&mut self, chapter_index: usize) -> Result<String> {
        if chapter_index >= self.doc.spine.len() {
            return Err(anyhow::anyhow!("Chapter index out of bounds"));
        }

        self.doc.set_current_chapter(chapter_index);
        let content = self
            .doc
            .get_current_with_epub_uris()
            .context("Failed to get chapter content")?;

        if content.is_empty() {
            return Ok(" [ No content in this chapter ] ".to_string());
        }

        // Convert HTML to plain text
        // Increased width for better terminal usage
        let text = from_read(&content[..], 120);
        let mut result = text.context("Failed to convert HTML to text")?;

        if result.trim().is_empty() {
            result = " [ Chapter contains no renderable text ] ".to_string();
        }

        Ok(result)
    }

    pub fn get_toc(&self) -> Vec<String> {
        if self.doc.toc.is_empty() {
            // Fallback: list chapters by index
            (0..self.doc.spine.len())
                .map(|i| format!("Chapter {}", i + 1))
                .collect()
        } else {
            self.doc.toc.iter().map(|t| t.label.clone()).collect()
        }
    }
}
