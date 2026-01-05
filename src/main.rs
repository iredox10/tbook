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
use std::{io, time::{Duration, Instant}};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let app = App::new("tbook.db")?;

    if args.len() > 2 && args[1] == "dict" {
        let res = App::perform_lookup(args[2].clone()).await;
        println!("{}", res);
        return Ok(());
    }

    if args.len() > 2 && args[1] == "export" {
        let book_id = args[2].parse::<i32>()?;
        let db = db::Db::new("tbook.db")?;
        let annos = db.get_annotations(book_id)?;
        let mut output = format!("# Exported Annotations\n\n");
        for a in annos {
            output.push_str(&format!("## Chapter {}\n", a.chapter + 1));
            output.push_str(&format!("> {}\n\n", a.content));
            if let Some(n) = a.note { output.push_str(&format!("**Note:** {}\n\n", n)); }
            output.push_str("---\n\n");
        }
        std::fs::write("export.md", output)?;
        println!("Exported to export.md");
        return Ok(());
    }

    if args.len() > 2 && args[1] == "add" {
        let path = &args[2];
        let parser = if path.to_lowercase().ends_with(".pdf") {
            parser::BookParser::Pdf(parser::PdfParser::new(path)?)
        } else {
            parser::BookParser::Epub(parser::EpubParser::new(path)?)
        };
        let (title, author) = parser.get_metadata();
        app.db.add_book(&title, &author, path)?;
        println!("Added: {} by {}", title, author);
        return Ok(());
    }

    if args.len() > 1 && args[1] == "list" {
        for b in app.books {
            println!("ID: {}, Title: {}, Author: {}, Path: {}", b.id, b.title, b.author, b.path);
        }
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let res = run_app(&mut terminal, app).await;

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

async fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    let mut last_rsvp_tick = Instant::now();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);

    loop {
        let terminal_size = terminal.size().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let viewport_height = (terminal_size.height as usize).saturating_sub(1); 

        terminal
            .draw(|f| ui::render(f, &mut app))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        // Check for dictionary results
        if let Ok(res) = rx.try_recv() {
            app.dictionary_result = res.clone();
            app.db.add_to_vocabulary(&app.dictionary_query, &res).ok();
        }

        // Handle RSVP timing
        if app.view == AppView::Rsvp && app.rsvp_active {
            let interval = Duration::from_millis(60_000 / app.rsvp_wpm);
            if last_rsvp_tick.elapsed() >= interval {
                if app.rsvp_index + 1 < app.rsvp_words.len() {
                    app.rsvp_index += 1;
                } else {
                    app.rsvp_active = false;
                }
                last_rsvp_tick = Instant::now();
            }
        }

        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match app.view {
                    AppView::Library => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('S') => {
                            app.global_search_query.clear();
                            app.global_search_results.clear();
                            app.view = AppView::GlobalSearch;
                        }
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
                                eprintln!("Error opening book: {:?}", e);
                            }
                        }
                        _ => {}
                    },
                    AppView::GlobalSearch => match key.code {
                        KeyCode::Esc => app.view = AppView::Library,
                        KeyCode::Enter => {
                            if !app.global_search_results.is_empty() {
                                let res = &app.global_search_results[app.selected_search_index];
                                let book_id = res.0;
                                let chapter = res.2;
                                if let Some(idx) = app.books.iter().position(|b| b.id == book_id) {
                                    app.selected_book_index = idx;
                                    let _ = app.open_selected_book();
                                    if let Some(ref mut book) = app.current_book {
                                        book.current_chapter = chapter;
                                        let content = book.parser.get_chapter_content(chapter).unwrap_or_default();
                                        book.chapter_content = content.lines().map(|s| s.to_string()).collect();
                                    }
                                }
                            } else {
                                if let Ok(results) = app.global_search(&app.global_search_query.clone()) {
                                    app.global_search_results = results;
                                    app.selected_search_index = 0;
                                }
                            }
                        }
                        KeyCode::Char(c) => app.global_search_query.push(c),
                        KeyCode::Backspace => { app.global_search_query.pop(); }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.global_search_results.is_empty() {
                                app.selected_search_index = (app.selected_search_index + 1) % app.global_search_results.len();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.global_search_results.is_empty() {
                                if app.selected_search_index > 0 {
                                    app.selected_search_index -= 1;
                                } else {
                                    app.selected_search_index = app.global_search_results.len() - 1;
                                }
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
                        KeyCode::Char('s') => {
                            app.view = AppView::Select;
                        }
                        KeyCode::Char('A') => {
                            let _ = app.load_annotations();
                        }
                        KeyCode::Char('V') => {
                            let _ = app.load_vocabulary();
                        }
                        KeyCode::Char('E') => {
                            let _ = app.export_annotations();
                        }
                        KeyCode::Char('t') => {
                            app.open_toc();
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_viewport_down(),
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_viewport_up(),
                        KeyCode::Right | KeyCode::Char('l') => { let _ = app.next_chapter(); }
                        KeyCode::Left | KeyCode::Char('h') => { let _ = app.prev_chapter(); }
                        KeyCode::Char('c') => app.toggle_theme(),
                        KeyCode::Char('[') => app.adjust_margin(-1),
                        KeyCode::Char(']') => app.adjust_margin(1),
                        KeyCode::Char('{') => app.adjust_spacing(1),
                        KeyCode::Char('}') => app.adjust_spacing(-1),
                        KeyCode::Char('/') => {
                            app.view = AppView::Search;
                            app.search_query.clear();
                        }
                        _ => {}
                    },
                    AppView::Select | AppView::Visual => match key.code {
                        KeyCode::Char('v') => {
                            if app.view == AppView::Visual {
                                app.exit_visual_mode();
                            } else {
                                app.enter_visual_mode();
                            }
                        }
                        KeyCode::Char('a') => {
                            app.annotation_note.clear();
                            app.view = AppView::Annotation;
                        }
                        KeyCode::Char('h') => {
                            if app.view == AppView::Visual {
                                let _ = app.add_quick_highlight();
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(ref book) = app.current_book {
                                if let Some(line) = book.chapter_content.get(book.current_line) {
                                    if let Some(word) = line.split_whitespace().nth(book.word_index) {
                                        let clean_word: String = word.chars().filter(|c| c.is_alphabetic()).collect();
                                        if !clean_word.is_empty() {
                                            app.dictionary_query = clean_word.clone();
                                            app.view = AppView::Dictionary;
                                            app.dictionary_result = "Loading definition...".to_string();
                                            let tx_clone = tx.clone();
                                            tokio::spawn(async move {
                                                let result = App::perform_lookup(clean_word).await;
                                                let _ = tx_clone.send(result).await;
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => app.move_cursor_down(viewport_height),
                        KeyCode::Up | KeyCode::Char('k') => app.move_cursor_up(),
                        KeyCode::Char('w') => app.cursor_right(viewport_height),
                        KeyCode::Char('b') => app.cursor_left(),
                        KeyCode::Esc => {
                            if app.view == AppView::Visual {
                                app.exit_visual_mode();
                            } else {
                                app.view = AppView::Reader;
                            }
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
                        KeyCode::Enter => { let _ = app.jump_to_toc(); }
                        _ => {}
                    },
                    AppView::Annotation => match key.code {
                        KeyCode::Enter => { let _ = app.add_annotation_with_note(); }
                        KeyCode::Esc => app.view = AppView::Select,
                        KeyCode::Char(c) => app.annotation_note.push(c),
                        KeyCode::Backspace => { app.annotation_note.pop(); }
                        _ => {}
                    },
                    AppView::AnnotationList => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.view = AppView::Reader,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.current_annotations.is_empty() {
                                app.selected_annotation_index = (app.selected_annotation_index + 1) % app.current_annotations.len();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.current_annotations.is_empty() {
                                if app.selected_annotation_index > 0 {
                                    app.selected_annotation_index -= 1;
                                } else {
                                    app.selected_annotation_index = app.current_annotations.len() - 1;
                                }
                            }
                        }
                        KeyCode::Enter => { let _ = app.jump_to_annotation(); }
                        _ => {}
                    },
                    AppView::Dictionary => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.view = AppView::Select,
                        _ => {}
                    },
                    AppView::Vocabulary => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.view = AppView::Reader,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.vocabulary.is_empty() {
                                app.selected_vocab_index = (app.selected_vocab_index + 1) % app.vocabulary.len();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.vocabulary.is_empty() {
                                if app.selected_vocab_index > 0 {
                                    app.selected_vocab_index -= 1;
                                } else {
                                    app.selected_vocab_index = app.vocabulary.len() - 1;
                                }
                            }
                        }
                        _ => {}
                    },
                    AppView::Search => match key.code {
                        KeyCode::Enter => {
                            if let Some(ref book) = app.current_book {
                                if let Some(pos) = book.chapter_content.iter().skip(book.current_line + 1).position(|l| l.contains(&app.search_query)) {
                                    for _ in 0..(pos + 1) { app.move_cursor_down(viewport_height); }
                                }
                            }
                            app.view = AppView::Reader;
                        }
                        KeyCode::Esc => app.view = AppView::Reader,
                        KeyCode::Char(c) => app.search_query.push(c),
                        KeyCode::Backspace => { app.search_query.pop(); }
                        _ => {}
                    },
                    AppView::Rsvp => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.rsvp_active = false;
                            app.view = AppView::Reader;
                        }
                        KeyCode::Char(' ') => {
                            app.rsvp_active = !app.rsvp_active;
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => app.rsvp_wpm += 50,
                        KeyCode::Char('-') => if app.rsvp_wpm > 50 { app.rsvp_wpm -= 50; }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
