mod app;
mod db;
mod parser;
mod ui;

use anyhow::Result;
use app::{App, AppView};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let app = App::new("tbook.db")?;

    if args.len() > 2 && args[1] == "test-parse" {
        let path = &args[2];
        let mut parser = parser::EpubParser::new(path)?;
        let count = parser.get_chapter_count();
        println!("Chapters: {}", count);
        for i in 0..count {
            let content = parser.get_chapter_content(i)?;
            println!("Chapter {}: {} chars", i, content.len());
            if i < 3 {
                println!("--- START ---");
                println!("{}", content);
                println!("--- END ---");
            }
        }
        return Ok(());
    }

    if args.len() > 1 && args[1] == "list" {
        for b in app.books {
            println!("ID: {}, Title: {}, Author: {}, Path: {}", b.id, b.title, b.author, b.path);
        }
        return Ok(());
    }

    if args.len() > 2 && args[1] == "add" {
        let path = &args[2];
        let parser = parser::EpubParser::new(path)?;
        let (title, author) = parser.get_metadata();
        app.db.add_book(&title, &author, path)?;
        println!("Added: {} by {}", title, author);
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal
            .draw(|f| ui::render(f, &mut app))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.view {
                    AppView::Library => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.books.is_empty() {
                                app.selected_book_index = (app.selected_book_index + 1) % app.books.len();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.books.is_empty() {
                                if app.selected_book_index > 0 {
                                    app.selected_book_index -= 1;
                                } else {
                                    app.selected_book_index = app.books.len() - 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if let Err(e) = app.open_selected_book() {
                                // In a real app, show error in UI
                                eprintln!("Error opening book: {:?}", e);
                            }
                        }
                        _ => {}
                    },
                    AppView::Reader => match key.code {
                        KeyCode::Char('q') => {
                            app.save_progress().ok();
                            app.view = AppView::Library;
                            app.refresh_library().ok();
                        }
                        KeyCode::Char('/') => {
                            app.view = AppView::Search;
                            app.search_query.clear();
                        }
                        KeyCode::Char('t') => {
                            app.open_toc();
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                        KeyCode::Right | KeyCode::Char('l') => {
                            let _ = app.next_chapter();
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            let _ = app.prev_chapter();
                        }
                        _ => {}
                    },
                    AppView::Toc => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.view = AppView::Reader,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.toc_items.is_empty() {
                                app.selected_toc_index = (app.selected_toc_index + 1) % app.toc_items.len();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.toc_items.is_empty() {
                                if app.selected_toc_index > 0 {
                                    app.selected_toc_index -= 1;
                                } else {
                                    app.selected_toc_index = app.toc_items.len() - 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let _ = app.jump_to_toc();
                        }
                        _ => {}
                    },
                    AppView::Search => match key.code {
                        KeyCode::Enter => {
                            // Simple search: find first match in current chapter
                            if let Some(ref book) = app.current_book {
                                if let Some(pos) = book.chapter_content.iter().skip(book.current_line + 1).position(|l| l.contains(&app.search_query)) {
                                    app.scroll_down();
                                    for _ in 0..pos {
                                        app.scroll_down();
                                    }
                                }
                            }
                            app.view = AppView::Reader;
                        }
                        KeyCode::Esc => app.view = AppView::Reader,
                        KeyCode::Char(c) => app.search_query.push(c),
                        KeyCode::Backspace => {
                            app.search_query.pop();
                        }
                        _ => {}
                    },
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
