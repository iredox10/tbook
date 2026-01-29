use anyhow::{Context, Result};
use pdf::file::FileOptions;
use std::fs;
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

    pub fn get_cover_image(&self) -> Result<image::DynamicImage> {
        // Use the first page as a reasonable "cover".
        self.render_page_image(1)
    }

    pub fn get_chapter_content(&mut self, index: usize) -> Result<Vec<crate::parser::PageContent>> {
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
            // For scanned/image-based PDFs, fall back to rendering the page as an image.
            // Requires `pdftoppm` from poppler-utils.
            match self.render_page_image(page_num) {
                Ok(img) => Ok(vec![crate::parser::PageContent::Image(
                    std::sync::Arc::new(img),
                )]),
                Err(_) => Ok(vec![crate::parser::PageContent::Text(
                    " [ Blank Page or Text Not Extractable ] ".to_string(),
                )]),
            }
        } else {
            Ok(vec![crate::parser::PageContent::Text(text)])
        }
    }

    fn render_page_image(&self, page_num: usize) -> Result<image::DynamicImage> {
        let tmp = std::env::temp_dir();
        let unique = format!(
            "tbook_pdf_{}_{}_{}",
            std::process::id(),
            page_num,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let root = tmp.join(unique);
        let root_str = root
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("temp path not valid utf-8"))?;

        let output = Command::new("pdftoppm")
            .args(&[
                "-f",
                &page_num.to_string(),
                "-l",
                &page_num.to_string(),
                "-png",
                "-singlefile",
                "-r",
                "150",
                &self.path,
                root_str,
            ])
            .output()
            .context("Failed to execute pdftoppm. Ensure poppler-utils is installed.")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("pdftoppm failed: {}", stderr));
        }

        let png_path = root.with_extension("png");
        let bytes = fs::read(&png_path)
            .with_context(|| format!("Failed to read rendered page image: {:?}", png_path))?;
        let _ = fs::remove_file(&png_path);
        let img = image::load_from_memory(&bytes).context("Failed to decode rendered PDF page")?;
        Ok(img)
    }

    pub fn get_toc(&self) -> Vec<String> {
        (0..self.page_count)
            .map(|i| format!("Page {}", i + 1))
            .collect()
    }
}
