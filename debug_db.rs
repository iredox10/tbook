use rusqlite::Connection;

fn main() -> anyhow::Result<()> {
    let conn = Connection::open("tbook.db")?;
    let mut stmt = conn.prepare("SELECT id, title, author, path FROM books")?;
    let book_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    for book in book_iter {
        let (id, title, author, path) = book?;
        println!(
            "ID: {}, Title: {}, Author: {}, Path: {}",
            id, title, author, path
        );
    }
    Ok(())
}
