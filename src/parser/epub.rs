use crate::parser::PageContent;
use anyhow::{Context, Result};
use epub::doc::EpubDoc;
use html2text::from_read;
use regex::Regex;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

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

    pub fn get_chapter_content(&mut self, chapter_index: usize) -> Result<Vec<PageContent>> {
        if chapter_index >= self.doc.spine.len() {
            return Err(anyhow::anyhow!("Chapter index out of bounds"));
        }

        self.doc.set_current_chapter(chapter_index);
        let content_bytes = self
            .doc
            .get_current_with_epub_uris()
            .context("Failed to get chapter content")?;

        if content_bytes.is_empty() {
            return Ok(vec![PageContent::Text(
                " [ No content in this chapter ] ".to_string(),
            )]);
        }

        let content_str = String::from_utf8_lossy(&content_bytes);

        let mut result_items = Vec::new();
        // Regex to find <img> or <image> tags with src/href
        // NOTE: This is a simplistic regex approach and might fail on complex HTML, but works for most EPUBs
        let re = Regex::new(
            r#"(?i)<(?:img[^>]+src=["']([^"']+)["']|image[^>]+href=["']([^"']+)["'])[^>]*>"#,
        )
        .unwrap();

        let mut last_pos = 0;

        for cap in re.captures_iter(&content_str) {
            let match_start = cap.get(0).unwrap().start();
            let match_end = cap.get(0).unwrap().end();

            // Extract text before image
            if match_start > last_pos {
                let text_html = &content_str[last_pos..match_start];
                // Wrap in div to ensure block context if it was a fragment
                let wrapped_html = format!("<div>{}</div>", text_html);
                let plain_text_res = from_read(wrapped_html.as_bytes(), 120);
                if let Ok(plain_text) = plain_text_res {
                    if !plain_text.trim().is_empty() {
                        result_items.push(PageContent::Text(plain_text));
                    }
                }
            }

            // Extract Image
            let src = cap.get(1).or(cap.get(2)).map(|m| m.as_str()).unwrap_or("");
            if !src.is_empty() {
                // Try direct match
                let mut img_data = self.doc.get_resource(src);

                // If not found, try to resolve relative path or just match by filename
                if img_data.is_none() {
                    let filename = Path::new(src)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    if !filename.is_empty() {
                        // Search in all resources for this filename
                        let found_key = self
                            .doc
                            .resources
                            .keys()
                            .find(|k| k.ends_with(filename))
                            .cloned();
                        if let Some(key) = found_key {
                            img_data = self.doc.get_resource(&key);
                        }
                    }
                }

                if let Some((img_bytes, _)) = img_data {
                    if let Ok(img) = image::load_from_memory(&img_bytes) {
                        result_items.push(PageContent::Image(Arc::new(img)));
                    } else {
                        result_items.push(PageContent::Text(format!(
                            "[ Error decoding image: {} ]",
                            src
                        )));
                    }
                } else {
                    result_items.push(PageContent::Text(format!(
                        "[ Image resource not found: {} ]",
                        src
                    )));
                }
            }

            last_pos = match_end;
        }

        // Remaining text
        if last_pos < content_str.len() {
            let text_html = &content_str[last_pos..];
            let wrapped_html = format!("<div>{}</div>", text_html);
            let plain_text_res = from_read(wrapped_html.as_bytes(), 120);
            if let Ok(plain_text) = plain_text_res {
                if !plain_text.trim().is_empty() {
                    result_items.push(PageContent::Text(plain_text));
                }
            }
        }

        if result_items.is_empty() {
            result_items.push(PageContent::Text(
                " [ Chapter contains no renderable text ] ".to_string(),
            ));
        }

        Ok(result_items)
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

    pub fn get_cover(&mut self) -> Option<image::DynamicImage> {
        self.doc
            .get_cover()
            .and_then(|(img_bytes, _)| image::load_from_memory(&img_bytes).ok())
    }
}
