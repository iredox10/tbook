mod app;
mod db;
mod parser;
mod ui;

use parser::EpubParser;

fn main() -> anyhow::Result<()> {
    let mut parser = EpubParser::new("/home/iredox/filezilla/test.epub")?;
    println!("Title: {}, Author: {}", parser.get_metadata().0, parser.get_metadata().1);
    let chapters = parser.get_chapter_count();
    println!("Total chapters: {}", chapters);
    
    for i in 0..chapters {
        match parser.get_chapter_content(i) {
            Ok(content) => {
                println!("Chapter {}: {} chars", i, content.len());
                if content.len() > 0 {
                    println!("First 100 chars: {}", &content[..std::cmp::min(100, content.len())]);
                }
            }
            Err(e) => println!("Chapter {}: Error: {:?}", i, e),
        }
    }
    Ok(())
}
