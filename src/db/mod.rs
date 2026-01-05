use rusqlite::{params, Connection, Result};
use std::path::Path;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    fn init(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS books (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                author TEXT,
                path TEXT NOT NULL UNIQUE,
                current_chapter INTEGER DEFAULT 0,
                current_line INTEGER DEFAULT 0,
                total_chapters INTEGER DEFAULT 0,
                last_read TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS annotations (
                id INTEGER PRIMARY KEY,
                book_id INTEGER NOT NULL,
                chapter INTEGER NOT NULL,
                start_line INTEGER NOT NULL,
                start_word INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                end_word INTEGER NOT NULL,
                content TEXT NOT NULL,
                note TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(book_id) REFERENCES books(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS vocabulary (
                id INTEGER PRIMARY KEY,
                word TEXT NOT NULL UNIQUE,
                definition TEXT,
                lookup_count INTEGER DEFAULT 1,
                last_lookup TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(())
    }

    pub fn add_book(&self, title: &str, author: &str, path: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO books (title, author, path) VALUES (?1, ?2, ?3)",
            params![title, author, path],
        )?;
        Ok(())
    }

    pub fn get_books(&self) -> Result<Vec<BookRecord>> {
        let mut stmt = self.conn.prepare("SELECT id, title, author, path, current_chapter, current_line FROM books ORDER BY last_read DESC")?;
        let book_iter = stmt.query_map([], |row| {
            let current_chapter: i32 = row.get(4)?;
            let current_line: i32 = row.get(5)?;
            Ok(BookRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                path: row.get(3)?,
                current_chapter: current_chapter as usize,
                current_line: current_line as usize,
            })
        })?;

        let mut books = Vec::new();
        for book in book_iter {
            books.push(book?);
        }
        Ok(books)
    }

    pub fn update_progress(&self, path: &str, chapter: usize, line: usize) -> Result<()> {
        self.conn.execute(
            "UPDATE books SET current_chapter = ?1, current_line = ?2, last_read = CURRENT_TIMESTAMP WHERE path = ?3",
            params![chapter as i32, line as i32, path],
        )?;
        Ok(())
    }

    pub fn add_annotation(
        &self,
        book_id: i32,
        chapter: usize,
        start_line: usize,
        start_word: usize,
        end_line: usize,
        end_word: usize,
        content: &str,
        note: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO annotations (book_id, chapter, start_line, start_word, end_line, end_word, content, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                book_id,
                chapter as i32,
                start_line as i32,
                start_word as i32,
                end_line as i32,
                end_word as i32,
                content,
                note
            ],
        )?;
        Ok(())
    }

    pub fn get_annotations(&self, book_id: i32) -> Result<Vec<AnnotationRecord>> {
        let mut stmt = self.conn.prepare("SELECT id, chapter, start_line, start_word, end_line, end_word, content, note FROM annotations WHERE book_id = ?1 ORDER BY chapter, start_line, start_word")?;
        let anno_iter = stmt.query_map(params![book_id], |row| {
            let chapter: i32 = row.get(1)?;
            let start_line: i32 = row.get(2)?;
            let start_word: i32 = row.get(3)?;
            let end_line: i32 = row.get(4)?;
            let end_word: i32 = row.get(5)?;
            Ok(AnnotationRecord {
                id: row.get(0)?,
                chapter: chapter as usize,
                start_line: start_line as usize,
                start_word: start_word as usize,
                end_line: end_line as usize,
                end_word: end_word as usize,
                content: row.get(6)?,
                note: row.get(7)?,
            })
        })?;

        let mut annos = Vec::new();
        for anno in anno_iter {
            annos.push(anno?);
        }
        Ok(annos)
    }

    pub fn add_to_vocabulary(&self, word: &str, definition: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO vocabulary (word, definition) VALUES (?1, ?2)
             ON CONFLICT(word) DO UPDATE SET 
                lookup_count = lookup_count + 1,
                last_lookup = CURRENT_TIMESTAMP",
            params![word, definition],
        )?;
        Ok(())
    }

    pub fn get_vocabulary(&self) -> Result<Vec<VocabRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT word, definition, lookup_count FROM vocabulary ORDER BY last_lookup DESC",
        )?;
        let vocab_iter = stmt.query_map([], |row| {
            Ok(VocabRecord {
                word: row.get(0)?,
                definition: row.get(1)?,
                lookup_count: row.get(2)?,
            })
        })?;

        let mut vocab = Vec::new();
        for v in vocab_iter {
            vocab.push(v?);
        }
        Ok(vocab)
    }
}

pub struct BookRecord {
    pub id: i32,
    pub title: String,
    pub author: String,
    pub path: String,
    pub current_chapter: usize,
    pub current_line: usize,
}

#[derive(Clone, Debug)]
pub struct AnnotationRecord {
    pub id: i32,
    pub chapter: usize,
    pub start_line: usize,
    pub start_word: usize,
    pub end_line: usize,
    pub end_word: usize,
    pub content: String,
    pub note: Option<String>,
}

pub struct VocabRecord {
    pub word: String,
    pub definition: String,
    pub lookup_count: i32,
}
