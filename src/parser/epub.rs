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
        // Regex to find inline images.
        // Covers common EPUB patterns:
        // - <img src="...">
        // - <img srcset="...">
        // - <image href="..."> (SVG)
        // - <image xlink:href="..."> (SVG)
        // NOTE: This is still a best-effort regex approach and may miss CSS background images.
        let re = Regex::new(
            r#"(?i)<(?:img[^>]+(?:src=["']([^"']+)["']|srcset=["']([^"']+)["'])|image[^>]+(?:href=["']([^"']+)["']|xlink:href=["']([^"']+)["']))[^>]*>"#,
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
            let mut src = cap
                .get(1)
                .or(cap.get(2))
                .or(cap.get(3))
                .or(cap.get(4))
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string();

            // If this was a srcset, take the first URL.
            // Format is typically: "url1 1x, url2 2x" or "url1 300w, url2 600w".
            if src.contains(',') || src.contains(' ') {
                let first = src
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim();
                if !first.is_empty() {
                    src = first.to_string();
                }
            }
            if !src.is_empty() {
                let mut resolved_bytes: Option<Vec<u8>> = None;

                // `get_current_with_epub_uris()` rewrites img/image URLs into `epub://<path>`.
                // Prefer fetching by full archive path since `get_resource()` expects a manifest id.
                if let Some(rest) = src.strip_prefix("epub://") {
                    let path_str = rest
                        .split('#')
                        .next()
                        .unwrap_or(rest)
                        .split('?')
                        .next()
                        .unwrap_or(rest);
                    resolved_bytes = self.doc.get_resource_by_path(path_str);
                }

                // Try manifest id match
                let mut img_data = if resolved_bytes.is_none() {
                    self.doc.get_resource(&src)
                } else {
                    None
                };

                // If not found, try to resolve by filename against manifest paths.
                if resolved_bytes.is_none() && img_data.is_none() {
                    let filename = Path::new(&src)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    if !filename.is_empty() {
                        // Search in all resources for a path ending with this filename.
                        let found_path = self.doc.resources.values().find_map(|r| {
                            let p = r.path.to_string_lossy();
                            if p.ends_with(filename) {
                                Some(r.path.clone())
                            } else {
                                None
                            }
                        });
                        if let Some(p) = found_path {
                            resolved_bytes = self.doc.get_resource_by_path(p);
                        }
                    }
                }

                let img_bytes_opt = resolved_bytes.or_else(|| img_data.take().map(|(b, _)| b));

                if let Some(img_bytes) = img_bytes_opt {
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

    pub fn get_cover_best_effort(&mut self) -> Option<image::DynamicImage> {
        if let Some(img) = self.get_cover() {
            return Some(img);
        }

        // Fallback: pick the largest image that looks like a cover.
        // Many EPUBs don't properly mark the cover in metadata.
        let mut best: Option<(u32, image::DynamicImage)> = None;

        let candidates: Vec<(std::path::PathBuf, String)> = self
            .doc
            .resources
            .values()
            .map(|r| (r.path.clone(), r.mime.clone()))
            .collect();

        for (path, mime) in candidates {
            if !mime.starts_with("image/") {
                continue;
            }

            let p = path.to_string_lossy().to_lowercase();
            if !(p.contains("cover") || p.contains("front")) {
                continue;
            }

            let Some(bytes) = self.doc.get_resource_by_path(&path) else {
                continue;
            };
            if let Ok(img) = image::load_from_memory(&bytes) {
                let score = img.width().saturating_mul(img.height());
                match &best {
                    Some((best_score, _)) if *best_score >= score => {}
                    _ => best = Some((score, img)),
                }
            }
        }

        best.map(|(_, img)| img)
    }
}
