use crate::db::{BookRecord, Db};
use crate::parser::EpubParser;
use anyhow::Result;

pub enum AppView {
    Library,
    Reader,
    Search,
    Toc,
}

pub struct App {
    pub view: AppView,
    pub db: Db,
    pub books: Vec<BookRecord>,
    pub selected_book_index: usize,
    pub current_book: Option<LoadedBook>,
    pub should_quit: bool,
    pub search_query: String,
    pub toc_items: Vec<String>,
    pub selected_toc_index: usize,
}

pub struct LoadedBook {
    pub parser: EpubParser,
    pub path: String,
    pub current_chapter: usize,
    pub current_line: usize,
    pub chapter_content: Vec<String>, // Lines of current chapter
}

impl App {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Db::new(db_path)?;
        let books = db.get_books()?;
        Ok(Self {
            view: AppView::Library,
            db,
            books,
            selected_book_index: 0,
            current_book: None,
            should_quit: false,
            search_query: String::new(),
            toc_items: Vec::new(),
            selected_toc_index: 0,
        })
    }

    pub fn refresh_library(&mut self) -> Result<()> {
        self.books = self.db.get_books()?;
        Ok(())
    }

    pub fn open_selected_book(&mut self) -> Result<()> {
        if self.books.is_empty() {
            return Ok(());
        }
        let book_record = &self.books[self.selected_book_index];
        let mut parser = EpubParser::new(&book_record.path)?;
        let content = parser.get_chapter_content(book_record.current_chapter)?;
        let chapter_content: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        self.current_book = Some(LoadedBook {
            parser,
            path: book_record.path.clone(),
            current_chapter: book_record.current_chapter,
            current_line: book_record.current_line,
            chapter_content,
        });
        self.view = AppView::Reader;
        Ok(())
    }

    pub fn next_chapter(&mut self) -> Result<()> {
        if let Some(ref mut book) = self.current_book {
            if book.current_chapter + 1 < book.parser.get_chapter_count() {
                book.current_chapter += 1;
                book.current_line = 0;
                let content = book.parser.get_chapter_content(book.current_chapter)?;
                book.chapter_content = content.lines().map(|s| s.to_string()).collect();
                self.save_progress()?;
            }
        }
        Ok(())
    }

    pub fn prev_chapter(&mut self) -> Result<()> {
        if let Some(ref mut book) = self.current_book {
            if book.current_chapter > 0 {
                book.current_chapter -= 1;
                book.current_line = 0;
                let content = book.parser.get_chapter_content(book.current_chapter)?;
                book.chapter_content = content.lines().map(|s| s.to_string()).collect();
                self.save_progress()?;
            }
        }
        Ok(())
    }

    pub fn save_progress(&mut self) -> Result<()> {
        if let Some(ref book) = self.current_book {
            self.db
                .update_progress(&book.path, book.current_chapter, book.current_line)?;
        }
        Ok(())
    }

    pub fn scroll_down(&mut self) {
        if let Some(ref mut book) = self.current_book {
            if book.current_line + 1 < book.chapter_content.len() {
                book.current_line += 1;
            }
        }
    }

    pub fn scroll_up(&mut self) {
        if let Some(ref mut book) = self.current_book {
            if book.current_line > 0 {
                book.current_line -= 1;
            }
        }
    }

    pub fn open_toc(&mut self) {
        if let Some(ref book) = self.current_book {
            self.toc_items = book.parser.get_toc();
            self.selected_toc_index = book.current_chapter;
            self.view = AppView::Toc;
        }
    }

    pub fn jump_to_toc(&mut self) -> Result<()> {
        if let Some(ref mut book) = self.current_book {
            book.current_chapter = self.selected_toc_index;
            book.current_line = 0;
            let content = book.parser.get_chapter_content(book.current_chapter)?;
            book.chapter_content = content.lines().map(|s| s.to_string()).collect();
            self.save_progress()?;
            self.view = AppView::Reader;
        }
        Ok(())
    }
}
