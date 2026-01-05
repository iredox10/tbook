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
}

pub struct BookRecord {
    pub id: i32,
    pub title: String,
    pub author: String,
    pub path: String,
    pub current_chapter: usize,
    pub current_line: usize,
}
