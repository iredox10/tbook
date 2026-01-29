use crate::db::{AnnotationRecord, BookRecord, Db, VocabRecord};
use crate::parser::{BookParser, EpubParser, PageContent, PdfParser};
use anyhow::Result;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use std::time::Instant;
use walkdir::WalkDir;

#[derive(Clone)]
pub enum RenderLine {
    Text(String),
    Image { protocol_idx: usize, row_idx: usize },
}

#[derive(PartialEq, Clone, Copy)]
pub enum AppView {
    Library,
    Reader,
    Search,
    Toc,
    #[allow(dead_code)]
    Rsvp,
    Annotation,
    AnnotationList,
    Dictionary,
    Visual,
    Select,
    Vocabulary,
    GlobalSearch,
    PathInput,
    FileExplorer,
    Help,
    Stats,
}

#[derive(Clone, Copy)]
pub enum Theme {
    Default,
    Gruvbox,
    Nord,
    Sepia,
}

pub struct App {
    pub view: AppView,
    pub previous_view: Option<AppView>,
    pub db: Db,
    pub books: Vec<BookRecord>,
    pub selected_book_index: usize,
    pub current_book: Option<LoadedBook>,
    pub should_quit: bool,
    pub search_query: String,
    pub toc_items: Vec<String>,
    pub selected_toc_index: usize,
    pub theme: Theme,
    // RSVP State
    pub rsvp_active: bool,
    pub rsvp_index: usize,
    pub rsvp_wpm: u64,
    pub rsvp_words: Vec<String>,
    // Annotation State
    pub annotation_note: String,
    pub current_annotations: Vec<AnnotationRecord>,
    pub selected_annotation_index: usize,
    // Dictionary State
    pub dictionary_query: String,
    pub dictionary_result: String,
    // Vocabulary State
    pub vocabulary: Vec<VocabRecord>,
    pub selected_vocab_index: usize,
    // Layout State
    pub margin: u16,
    pub line_spacing: u16,
    // Global Search State
    pub global_search_query: String,
    pub global_search_results: Vec<(i32, String, usize, String)>,
    pub selected_search_index: usize,
    // Explorer State
    pub explorer_path: String,
    pub explorer_results: Vec<std::path::PathBuf>,
    pub selected_explorer_index: usize,
    pub is_scanning: bool,
    pub image_picker: Picker,
    pub current_library_cover: Option<Box<dyn StatefulProtocol>>,
    // Auto-scroll State
    pub auto_scroll_active: bool,
    pub auto_scroll_interval_ms: u64,
    pub auto_scroll_last_tick: Instant,
}

pub struct LoadedBook {
    pub id: i32,
    pub parser: BookParser,
    pub path: String,
    pub current_chapter: usize,
    pub current_line: usize,              // Cursor line
    pub viewport_top: usize,              // Viewport top line
    pub chapter_content: Vec<RenderLine>, // Lines of current chapter
    pub image_protocols: Vec<Box<dyn StatefulProtocol>>,
    pub word_index: usize,                        // Cursor word index
    pub selection_anchor: Option<(usize, usize)>, // (line, word)
    pub chapter_annotations: Vec<AnnotationRecord>,
    pub start_time: Instant,
    pub words_read: usize,
    pub session_words_logged: usize,
}

impl App {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Db::new(db_path)?;
        let books = db.get_books()?;
        Ok(Self {
            view: AppView::Library,
            previous_view: None,
            db,
            books,
            selected_book_index: 0,
            current_book: None,
            should_quit: false,
            search_query: String::new(),
            toc_items: Vec::new(),
            selected_toc_index: 0,
            theme: Theme::Default,
            rsvp_active: false,
            rsvp_index: 0,
            rsvp_wpm: 300,
            rsvp_words: Vec::new(),
            annotation_note: String::new(),
            current_annotations: Vec::new(),
            selected_annotation_index: 0,
            dictionary_query: String::new(),
            dictionary_result: String::new(),
            vocabulary: Vec::new(),
            selected_vocab_index: 0,
            margin: 2,
            line_spacing: 0,
            global_search_query: String::new(),
            global_search_results: Vec::new(),
            selected_search_index: 0,
            explorer_path: String::new(),
            explorer_results: Vec::new(),
            selected_explorer_index: 0,
            is_scanning: false,
            image_picker: {
                let mut picker =
                    Picker::from_termios().unwrap_or_else(|_| Picker::new((8, 16)));
                // Default Picker protocol is Halfblocks (low-res). Guess a better protocol
                // (Kitty/Sixel/iTerm2) so covers and inline images render sharply.
                let _ = picker.guess_protocol();
                picker
            },
            current_library_cover: None,
            auto_scroll_active: false,
            auto_scroll_interval_ms: 2000, // Default scroll every 2 seconds
            auto_scroll_last_tick: Instant::now(),
        })
    }

    pub fn refresh_library(&mut self) -> Result<()> {
        self.books = self.db.get_books()?;
        self.load_selected_book_preview().ok();
        Ok(())
    }

    pub fn load_selected_book_preview(&mut self) -> Result<()> {
        self.current_library_cover = None;
        if self.books.is_empty() {
            return Ok(());
        }
        let book_record = &self.books[self.selected_book_index];
        if !book_record.path.to_lowercase().ends_with(".epub") {
            return Ok(());
        }

        let mut epub = EpubParser::new(&book_record.path)?;
        if let Some(cover) = epub.get_cover() {
            let protocol = self.image_picker.new_resize_protocol(cover);
            self.current_library_cover = Some(protocol);
        }
        Ok(())
    }

    pub fn open_selected_book(&mut self) -> Result<()> {
        if self.books.is_empty() {
            return Ok(());
        }
        let book_record = self.books[self.selected_book_index].clone();
        self.load_book(book_record)
    }

    pub fn load_book(&mut self, book_record: BookRecord) -> Result<()> {
        let mut parser = if book_record.path.to_lowercase().ends_with(".pdf") {
            BookParser::Pdf(PdfParser::new(&book_record.path)?)
        } else {
            BookParser::Epub(EpubParser::new(&book_record.path)?)
        };

        let content = parser.get_chapter_content(book_record.current_chapter)?;
        let (chapter_content, image_protocols) =
            Self::flatten_content(&mut self.image_picker, content);

        let chapter_annotations = self
            .db
            .get_annotations(book_record.id)?
            .into_iter()
            .filter(|a| a.chapter == book_record.current_chapter)
            .collect();

        self.current_book = Some(LoadedBook {
            id: book_record.id,
            parser,
            path: book_record.path.clone(),
            current_chapter: book_record.current_chapter,
            current_line: book_record.current_line,
            viewport_top: book_record.current_line,
            chapter_content,
            image_protocols,
            word_index: 0,
            selection_anchor: None,
            chapter_annotations,
            start_time: Instant::now(),
            words_read: 0,
            session_words_logged: 0,
        });
        self.db
            .update_progress(
                &book_record.path,
                book_record.current_chapter,
                book_record.current_line,
                0,
            )
            .ok();
        self.view = AppView::Reader;
        Ok(())
    }

    pub fn flatten_content(
        picker: &mut Picker,
        content: Vec<PageContent>,
    ) -> (Vec<RenderLine>, Vec<Box<dyn StatefulProtocol>>) {
        let mut lines = Vec::new();
        let mut protocols = Vec::new();
        for item in content {
            match item {
                PageContent::Text(s) => {
                    for line in s.lines() {
                        lines.push(RenderLine::Text(line.to_string()));
                    }
                }
                PageContent::Image(img) => {
                    let (w, h) = (img.width(), img.height());

                    // Aspect-ratio aware height calculation.
                    // Terminal cells are roughly 1:2 height:width ratio.
                    // We want to fit the image reasonably.
                    // Let's assume a default width of 80 characters for the reader.
                    let target_width_chars = 80;
                    let aspect_ratio = h as f32 / w as f32;
                    // height_chars = (target_width_chars * aspect_ratio) * cell_width_to_height_ratio
                    // typically cell_width_to_height_ratio is 0.5
                    let mut height_lines =
                        ((target_width_chars as f32 * aspect_ratio) * 0.5) as usize;

                    // Cap the height so it doesn't take over too many screens
                    height_lines = height_lines.clamp(5, 30);

                    let dynamic_image = (*img).clone();
                    let protocol = picker.new_resize_protocol(dynamic_image);
                    let protocol_idx = protocols.len();
                    protocols.push(protocol);
                    for i in 0..height_lines {
                        lines.push(RenderLine::Image {
                            protocol_idx,
                            row_idx: i,
                        });
                    }
                }
            }
        }
        if lines.is_empty() {
            lines.push(RenderLine::Text(" [ Empty ] ".to_string()));
        }
        (lines, protocols)
    }

    pub fn next_chapter(&mut self) -> Result<()> {
        let (should_update, new_chapter_idx) = if let Some(ref book) = self.current_book {
            if book.current_chapter + 1 < book.parser.get_chapter_count() {
                (true, book.current_chapter + 1)
            } else {
                (false, 0)
            }
        } else {
            (false, 0)
        };

        if should_update {
            if let Some(ref mut book) = self.current_book {
                book.current_chapter = new_chapter_idx;
                book.current_line = 0;
                book.viewport_top = 0;
                book.word_index = 0;
                book.selection_anchor = None;
            }

            let content = if let Some(ref mut book) = self.current_book {
                book.parser.get_chapter_content(new_chapter_idx)?
            } else {
                return Ok(());
            };

            let (flattened, protocols) = Self::flatten_content(&mut self.image_picker, content);

            let book_id = self.current_book.as_ref().unwrap().id;
            let chapter_annotations = self
                .db
                .get_annotations(book_id)?
                .into_iter()
                .filter(|a| a.chapter == new_chapter_idx)
                .collect();

            if let Some(ref mut book) = self.current_book {
                book.chapter_content = flattened;
                book.image_protocols = protocols;
                book.chapter_annotations = chapter_annotations;
            }
            self.save_progress()?;
        }
        Ok(())
    }

    pub fn prev_chapter(&mut self) -> Result<()> {
        let (should_update, new_chapter_idx) = if let Some(ref book) = self.current_book {
            if book.current_chapter > 0 {
                (true, book.current_chapter - 1)
            } else {
                (false, 0)
            }
        } else {
            (false, 0)
        };

        if should_update {
            if let Some(ref mut book) = self.current_book {
                book.current_chapter = new_chapter_idx;
                book.current_line = 0;
                book.viewport_top = 0;
                book.word_index = 0;
                book.selection_anchor = None;
            }

            let content = if let Some(ref mut book) = self.current_book {
                book.parser.get_chapter_content(new_chapter_idx)?
            } else {
                return Ok(());
            };

            let (flattened, protocols) = Self::flatten_content(&mut self.image_picker, content);

            let book_id = self.current_book.as_ref().unwrap().id;
            let chapter_annotations = self
                .db
                .get_annotations(book_id)?
                .into_iter()
                .filter(|a| a.chapter == new_chapter_idx)
                .collect();

            if let Some(ref mut book) = self.current_book {
                book.chapter_content = flattened;
                book.image_protocols = protocols;
                book.chapter_annotations = chapter_annotations;
            }
            self.save_progress()?;
        }
        Ok(())
    }

    pub fn save_progress(&mut self) -> Result<()> {
        if let Some(ref mut book) = self.current_book {
            self.db.update_progress(
                &book.path,
                book.current_chapter,
                book.current_line,
                book.words_read,
            )?;

            // Log session words
            let delta = book.words_read.saturating_sub(book.session_words_logged);
            if delta > 0 {
                self.db.log_reading_session(book.id, delta).ok();
                book.session_words_logged = book.words_read;
            }
        }
        Ok(())
    }

    pub fn scroll_viewport_down(&mut self) {
        if let Some(ref mut book) = self.current_book {
            if book.viewport_top + 1 < book.chapter_content.len() {
                book.viewport_top += 1;
                if let Some(RenderLine::Text(line)) =
                    book.chapter_content.get(book.viewport_top - 1)
                {
                    book.words_read += line.split_whitespace().count();
                }
                if book.current_line < book.viewport_top {
                    book.current_line = book.viewport_top;
                }
            }
        }
    }

    pub fn scroll_viewport_up(&mut self) {
        if let Some(ref mut book) = self.current_book {
            if book.viewport_top > 0 {
                book.viewport_top -= 1;
            }
        }
    }

    pub fn move_cursor_down(&mut self, height: usize) {
        if let Some(ref mut book) = self.current_book {
            if book.current_line + 1 < book.chapter_content.len() {
                book.current_line += 1;
                if book.current_line >= book.viewport_top + height.saturating_sub(2) {
                    book.viewport_top += 1;
                }
                Self::sync_word_index(book);
            }
        }
    }

    pub fn move_cursor_up(&mut self) {
        if let Some(ref mut book) = self.current_book {
            if book.current_line > 0 {
                book.current_line -= 1;
                if book.current_line < book.viewport_top {
                    book.viewport_top = book.current_line;
                }
                Self::sync_word_index(book);
            }
        }
    }

    fn sync_word_index(book: &mut LoadedBook) {
        match book.chapter_content.get(book.current_line) {
            Some(RenderLine::Text(_line)) => {
                let words = _line.split_whitespace().count();
                if book.word_index >= words && words > 0 {
                    book.word_index = words.saturating_sub(1);
                } else if words == 0 {
                    book.word_index = 0;
                }
            }
            Some(RenderLine::Image { .. }) => {
                book.word_index = 0;
            }
            None => {}
        }
    }

    pub fn cursor_right(&mut self, height: usize) {
        if let Some(ref mut book) = self.current_book {
            match book.chapter_content.get(book.current_line) {
                Some(RenderLine::Text(_line)) => {
                    let words: Vec<&str> = _line.split_whitespace().collect();
                    if book.word_index + 1 < words.len() {
                        book.word_index += 1;
                    } else if book.current_line + 1 < book.chapter_content.len() {
                        book.current_line += 1;
                        if book.current_line >= book.viewport_top + height.saturating_sub(2) {
                            book.viewport_top += 1;
                        }
                        book.word_index = 0;
                    }
                }
                Some(RenderLine::Image { .. }) => {
                    // Move to next line
                    if book.current_line + 1 < book.chapter_content.len() {
                        book.current_line += 1;
                        if book.current_line >= book.viewport_top + height.saturating_sub(2) {
                            book.viewport_top += 1;
                        }
                        book.word_index = 0;
                    }
                }
                None => {}
            }
        }
    }

    pub fn cursor_left(&mut self) {
        if let Some(ref mut book) = self.current_book {
            match book.chapter_content.get(book.current_line) {
                Some(RenderLine::Text(_line)) => {
                    if book.word_index > 0 {
                        book.word_index -= 1;
                    } else if book.current_line > 0 {
                        book.current_line -= 1;
                        if book.current_line < book.viewport_top {
                            book.viewport_top = book.current_line;
                        }
                        Self::sync_word_index(book);
                    }
                }
                Some(RenderLine::Image { .. }) => {
                    if book.current_line > 0 {
                        book.current_line -= 1;
                        if book.current_line < book.viewport_top {
                            book.viewport_top = book.current_line;
                        }
                        Self::sync_word_index(book);
                    }
                }
                None => {}
            }
        }
    }

    pub fn enter_visual_mode(&mut self) {
        if let Some(ref mut book) = self.current_book {
            if let Some(RenderLine::Text(_)) = book.chapter_content.get(book.current_line) {
                book.selection_anchor = Some((book.current_line, book.word_index));
                self.view = AppView::Visual;
            }
        }
    }

    pub fn exit_visual_mode(&mut self) {
        if let Some(ref mut book) = self.current_book {
            book.selection_anchor = None;
            if self.view == AppView::Visual {
                self.view = AppView::Select;
            }
        }
    }

    pub fn get_selection_range(&self) -> Option<(usize, usize, usize, usize)> {
        if let Some(ref book) = self.current_book {
            if let Some((anchor_line, anchor_word)) = book.selection_anchor {
                if anchor_line < book.current_line
                    || (anchor_line == book.current_line && anchor_word <= book.word_index)
                {
                    return Some((anchor_line, anchor_word, book.current_line, book.word_index));
                } else {
                    return Some((book.current_line, book.word_index, anchor_line, anchor_word));
                }
            }
        }
        None
    }

    pub fn get_selected_text(&self) -> String {
        if let Some((sl, sw, el, ew)) = self.get_selection_range() {
            if let Some(ref book) = self.current_book {
                let mut selected_words = Vec::new();
                for li in sl..=el {
                    if let Some(RenderLine::Text(line)) = book.chapter_content.get(li) {
                        let words: Vec<&str> = line.split_whitespace().collect();
                        let w_start = if li == sl { sw } else { 0 };
                        let w_end = if li == el {
                            std::cmp::min(ew, words.len().saturating_sub(1))
                        } else {
                            words.len().saturating_sub(1)
                        };

                        for wi in w_start..=w_end {
                            if let Some(w) = words.get(wi) {
                                selected_words.push(*w);
                            }
                        }
                    }
                }
                return selected_words.join(" ");
            }
        }
        String::new()
    }

    pub fn open_toc(&mut self) {
        if let Some(ref book) = self.current_book {
            self.toc_items = book.parser.get_toc();
            self.selected_toc_index = book.current_chapter;
            self.view = AppView::Toc;
        }
    }

    pub fn jump_to_toc(&mut self) -> Result<()> {
        let (should_jump, chapter_idx) = if let Some(ref _book) = self.current_book {
            (true, self.selected_toc_index)
        } else {
            (false, 0)
        };

        if should_jump {
            if let Some(ref mut book) = self.current_book {
                book.current_chapter = chapter_idx;
                book.current_line = 0;
                book.viewport_top = 0;
                book.word_index = 0;
                book.selection_anchor = None;
            }

            let content = if let Some(ref mut book) = self.current_book {
                book.parser.get_chapter_content(chapter_idx)?
            } else {
                return Ok(());
            };

            let (flattened, protocols) = Self::flatten_content(&mut self.image_picker, content);

            let book_id = self.current_book.as_ref().unwrap().id;
            let chapter_annotations = self
                .db
                .get_annotations(book_id)?
                .into_iter()
                .filter(|a| a.chapter == chapter_idx)
                .collect();

            if let Some(ref mut book) = self.current_book {
                book.chapter_content = flattened;
                book.image_protocols = protocols;
                book.chapter_annotations = chapter_annotations;
            }

            self.save_progress()?;
            self.view = AppView::Reader;
        }
        Ok(())
    }

    pub fn toggle_theme(&mut self) {
        self.theme = match self.theme {
            Theme::Default => Theme::Gruvbox,
            Theme::Gruvbox => Theme::Nord,
            Theme::Nord => Theme::Sepia,
            Theme::Sepia => Theme::Default,
        };
    }

    pub fn add_annotation_with_note(&mut self) -> Result<()> {
        let range = self.get_selection_range();
        let content = if range.is_some() {
            self.get_selected_text()
        } else if let Some(ref book) = self.current_book {
            if let Some(RenderLine::Text(line)) = book.chapter_content.get(book.current_line) {
                line.clone()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if let Some(ref mut book) = self.current_book {
            if let Some(RenderLine::Text(line)) = book.chapter_content.get(book.current_line) {
                let (sl, sw, el, ew) = range.unwrap_or((
                    book.current_line,
                    0,
                    book.current_line,
                    line.split_whitespace().count().saturating_sub(1),
                ));

                if !content.is_empty() {
                    let note = if self.annotation_note.trim().is_empty() {
                        None
                    } else {
                        Some(self.annotation_note.as_str())
                    };
                    self.db.add_annotation(
                        book.id,
                        book.current_chapter,
                        sl,
                        sw,
                        el,
                        ew,
                        &content,
                        note,
                    )?;
                    book.chapter_annotations = self
                        .db
                        .get_annotations(book.id)?
                        .into_iter()
                        .filter(|a| a.chapter == book.current_chapter)
                        .collect();
                }
            }
        }
        self.annotation_note.clear();
        self.exit_visual_mode();
        self.view = AppView::Reader;
        Ok(())
    }

    pub fn add_quick_highlight(&mut self) -> Result<()> {
        let range = self.get_selection_range();
        let content = self.get_selected_text();

        if let Some(ref mut book) = self.current_book {
            if let Some((sl, sw, el, ew)) = range {
                if !content.is_empty() {
                    self.db.add_annotation(
                        book.id,
                        book.current_chapter,
                        sl,
                        sw,
                        el,
                        ew,
                        &content,
                        None,
                    )?;
                    book.chapter_annotations = self
                        .db
                        .get_annotations(book.id)?
                        .into_iter()
                        .filter(|a| a.chapter == book.current_chapter)
                        .collect();
                }
            }
        }
        self.exit_visual_mode();
        Ok(())
    }

    pub fn load_annotations(&mut self) -> Result<()> {
        if let Some(ref book) = self.current_book {
            self.current_annotations = self.db.get_annotations(book.id)?;
            self.selected_annotation_index = 0;
            self.view = AppView::AnnotationList;
        }
        Ok(())
    }

    pub fn jump_to_annotation(&mut self) -> Result<()> {
        let (should_jump, chapter_idx, start_line, start_word) =
            if let Some(ref mut book) = self.current_book {
                if let Some(anno) = self.current_annotations.get(self.selected_annotation_index) {
                    if book.current_chapter != anno.chapter {
                        (true, anno.chapter, anno.start_line, anno.start_word)
                    } else {
                        // Same chapter, just move cursor
                        book.current_line = anno.start_line;
                        book.viewport_top = anno.start_line;
                        book.word_index = anno.start_word;
                        book.selection_anchor = None;
                        (false, 0, 0, 0)
                    }
                } else {
                    (false, 0, 0, 0)
                }
            } else {
                (false, 0, 0, 0)
            };

        if should_jump {
            if let Some(ref mut book) = self.current_book {
                book.current_chapter = chapter_idx;
                book.current_line = start_line;
                book.viewport_top = start_line;
                book.word_index = start_word;
                book.selection_anchor = None;
            }

            let content = if let Some(ref mut book) = self.current_book {
                book.parser.get_chapter_content(chapter_idx)?
            } else {
                return Ok(());
            };

            let (flattened, protocols) = Self::flatten_content(&mut self.image_picker, content);
            let book_id = self.current_book.as_ref().unwrap().id;
            let chapter_annotations = self
                .db
                .get_annotations(book_id)?
                .into_iter()
                .filter(|a| a.chapter == chapter_idx)
                .collect();

            if let Some(ref mut book) = self.current_book {
                book.chapter_content = flattened;
                book.image_protocols = protocols;
                book.chapter_annotations = chapter_annotations;
            }
            self.save_progress()?;
        }
        if self.current_book.is_some() && !self.current_annotations.is_empty() {
            self.view = AppView::Reader;
        }
        Ok(())
    }

    pub fn load_vocabulary(&mut self) -> Result<()> {
        self.vocabulary = self.db.get_vocabulary()?;
        self.selected_vocab_index = 0;
        self.view = AppView::Vocabulary;
        Ok(())
    }

    pub fn export_annotations(&self) -> Result<String> {
        if let Some(ref book) = self.current_book {
            let annos = self.db.get_annotations(book.id)?;
            let (title, author) = book.parser.get_metadata();

            let mut output = String::new();
            // YAML Frontmatter for Obsidian/Logseq
            output.push_str("---\n");
            output.push_str(&format!("title: \"{}\"\n", title));
            output.push_str(&format!("author: \"{}\"\n", author));
            output.push_str(&format!("source: \"{}\"\n", book.path));
            output.push_str(&format!(
                "exported: {}\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            ));
            output.push_str("tags: [tbook, reading-notes]\n");
            output.push_str("---\n\n");

            output.push_str(&format!("# Reading Notes: {}\n\n", title));

            for a in annos {
                output.push_str(&format!("### Chapter {}\n", a.chapter + 1));
                output.push_str(&format!("> {}\n", a.content.replace("\n", "\n> ")));
                if let Some(note) = a.note {
                    output.push_str(&format!("\n**Note:** {}\n", note));
                }
                output.push_str("\n---\n\n");
            }
            let filename = format!("notes_{}.md", title.to_lowercase().replace(" ", "_"));
            std::fs::write(&filename, output)?;
            Ok(filename)
        } else {
            Err(anyhow::anyhow!("No book open"))
        }
    }

    pub fn adjust_margin(&mut self, delta: i16) {
        let new_margin = (self.margin as i16) + delta;
        self.margin = new_margin.clamp(0, 20) as u16;
    }

    pub fn adjust_spacing(&mut self, delta: i16) {
        let new_spacing = (self.line_spacing as i16) + delta;
        self.line_spacing = new_spacing.clamp(0, 5) as u16;
    }

    pub fn get_reading_stats(&self) -> (usize, f64) {
        if let Some(ref book) = self.current_book {
            let elapsed = book.start_time.elapsed().as_secs_f64() / 60.0;
            let wpm = if elapsed > 0.01 {
                (book.words_read as f64) / elapsed
            } else {
                0.0
            };
            (book.words_read, wpm)
        } else {
            (0, 0.0)
        }
    }

    pub fn scan_for_books_sync(path: String) -> Vec<std::path::PathBuf> {
        let mut results = Vec::new();
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let f_path = entry.path();
            if f_path.is_file() {
                let ext = f_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ext == "epub" || ext == "pdf" {
                    results.push(f_path.to_path_buf());
                }
            }
        }
        results
    }

    pub fn global_search(&mut self, query: &str) -> Result<Vec<(i32, String, usize, String)>> {
        let mut results = Vec::new();
        let books = self.db.get_books()?;

        for book in books {
            let mut parser = if book.path.to_lowercase().ends_with(".pdf") {
                BookParser::Pdf(PdfParser::new(&book.path)?)
            } else {
                BookParser::Epub(EpubParser::new(&book.path)?)
            };
            let count = parser.get_chapter_count();
            for i in 0..count {
                if let Ok(content) = parser.get_chapter_content(i) {
                    let mut dummy_picker = Picker::new((8, 16));
                    let (lines, _) = Self::flatten_content(&mut dummy_picker, content);
                    for line_item in lines.iter() {
                        if let RenderLine::Text(line) = line_item {
                            if line.to_lowercase().contains(&query.to_lowercase()) {
                                results.push((
                                    book.id,
                                    book.title.clone(),
                                    i,
                                    line.trim().to_string(),
                                ));
                                if results.len() > 50 {
                                    return Ok(results);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    pub async fn perform_lookup(word: String) -> String {
        let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", word);
        let client = reqwest::Client::new();
        match client.get(url).send().await {
            Ok(resp) => {
                if let Ok(json_str) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        let mut result = String::new();
                        if let Some(entries) = json.as_array() {
                            for entry in entries {
                                if let Some(w) = entry.get("word").and_then(|v| v.as_str()) {
                                    result.push_str(&format!("# {}\n", w.to_uppercase()));
                                }
                                if let Some(meanings) =
                                    entry.get("meanings").and_then(|v| v.as_array())
                                {
                                    for meaning in meanings {
                                        if let Some(pos) =
                                            meaning.get("partOfSpeech").and_then(|v| v.as_str())
                                        {
                                            result.push_str(&format!("\n[{}]\n", pos));
                                        }
                                        if let Some(definitions) =
                                            meaning.get("definitions").and_then(|v| v.as_array())
                                        {
                                            for (i, def) in definitions.iter().enumerate() {
                                                if let Some(d) =
                                                    def.get("definition").and_then(|v| v.as_str())
                                                {
                                                    result.push_str(&format!("{}. {}\n", i + 1, d));
                                                }
                                            }
                                        }
                                    }
                                }
                                result.push_str("\n---\n");
                            }
                        }
                        if result.is_empty() {
                            "No definition found.".to_string()
                        } else {
                            result
                        }
                    } else {
                        "Failed to parse.".to_string()
                    }
                } else {
                    "Error reading response.".to_string()
                }
            }
            Err(e) => format!("Network Error: {}.", e),
        }
    }
}
