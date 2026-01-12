pub mod epub;
pub mod pdf;

pub use self::epub::EpubParser;
pub use self::pdf::PdfParser;

use anyhow::Result;
use image::DynamicImage;
use std::sync::Arc;

#[derive(Clone)]
pub enum PageContent {
    Text(String),
    Image(Arc<DynamicImage>),
}

pub enum BookParser {
    Epub(EpubParser),
    Pdf(PdfParser),
}

impl BookParser {
    pub fn get_metadata(&self) -> (String, String) {
        match self {
            BookParser::Epub(p) => p.get_metadata(),
            BookParser::Pdf(p) => p.get_metadata(),
        }
    }

    pub fn get_chapter_count(&self) -> usize {
        match self {
            BookParser::Epub(p) => p.get_chapter_count(),
            BookParser::Pdf(p) => p.get_chapter_count(),
        }
    }

    pub fn get_chapter_content(&mut self, index: usize) -> Result<Vec<PageContent>> {
        match self {
            BookParser::Epub(p) => p.get_chapter_content(index),
            BookParser::Pdf(p) => p.get_chapter_content(index),
        }
    }

    pub fn get_toc(&self) -> Vec<String> {
        match self {
            BookParser::Epub(p) => p.get_toc(),
            BookParser::Pdf(p) => p.get_toc(),
        }
    }

    // Removed get_total_lines as it was unused and caused overhead
}
