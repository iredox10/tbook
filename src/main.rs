mod app;
mod config;
mod db;
mod parser;
mod ui;

use anyhow::Result;
use app::{App, AppView};
use config::AppConfig;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui_image::picker::Picker;
use std::{io, time::Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let config = AppConfig::load().unwrap_or_default();
    let mut app = App::new("tbook.db")?;

    if args.len() > 2 && args[1] == "add" {
        let path = &args[2];
        add_book_to_db(&mut app, path)?;
        return Ok(());
    }

    if args.len() > 1 && args[1] == "list" {
        for b in app.books {
            println!(
                "ID: {}, Title: {}, Author: {}, Path: {}",
                b.id, b.title, b.author, b.path
            );
        }
        return Ok(());
    }

    if config.auto_resume && args.len() == 1 {
        if let Some(last_book) = app.db.get_last_read_book()? {
            app.load_book(last_book).ok();
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Query terminal capabilities (protocol + pixel cell size) after entering alt screen.
    // This improves Kitty/Ghostty image sharpness vs guessing.
    app.image_picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    app.load_selected_book_preview().ok();

    let res = run_app(&mut terminal, app).await;

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

fn add_book_to_db(app: &mut App, path: &str) -> Result<()> {
    let parser = if path.to_lowercase().ends_with(".pdf") {
        parser::BookParser::Pdf(parser::PdfParser::new(path)?)
    } else {
        parser::BookParser::Epub(parser::EpubParser::new(path)?)
    };
    let (title, author) = parser.get_metadata();
    let total_chapters = parser.get_chapter_count();
    let total_lines = 0;
    app.db
        .add_book(&title, &author, path, total_chapters, total_lines)?;
    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> Result<()> {
    let (tx_dict, mut rx_dict) = tokio::sync::mpsc::channel::<String>(10);
    let (tx_scan, mut rx_scan) = tokio::sync::mpsc::channel::<Vec<std::path::PathBuf>>(1);

    loop {
        let term_size = terminal
            .size()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let viewport_height = (term_size.height as usize).saturating_sub(1);

        terminal
            .draw(|f| ui::render(f, &mut app))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if let Ok(res) = rx_dict.try_recv() {
            app.dictionary_result = res.clone();
            app.db.add_to_vocabulary(&app.dictionary_query, &res).ok();
        }

        if let Ok(results) = rx_scan.try_recv() {
            app.explorer_results = results;
            app.is_scanning = false;
        }

        // Auto-scroll logic
        if app.view == AppView::Reader && app.auto_scroll_active {
            if app.auto_scroll_last_tick.elapsed().as_millis() as u64 >= app.auto_scroll_interval_ms
            {
                app.scroll_viewport_down();
                app.auto_scroll_last_tick = std::time::Instant::now();
            }
        }

        if event::poll(Duration::from_millis(10))? {
            let ev = event::read()?;
            if let Event::Mouse(mouse) = ev {
                if mouse.kind == event::MouseEventKind::Down(event::MouseButton::Left) {
                    if app.view == AppView::Reader {
                        let total_width = term_size.width;
                        if mouse.row == 0 {
                            // [ - ] area: x in [total_width - 14, total_width - 10]
                            if mouse.column >= total_width.saturating_sub(14)
                                && mouse.column <= total_width.saturating_sub(10)
                            {
                                app.adjust_margin(1); // Increase margin = decrease text width
                            }
                            // [ + ] area: x in [total_width - 7, total_width - 3]
                            if mouse.column >= total_width.saturating_sub(7)
                                && mouse.column <= total_width.saturating_sub(3)
                            {
                                app.adjust_margin(-1); // Decrease margin = increase text width
                            }
                        }
                    }
                }
            }

            if let Event::Key(key) = ev {
                if key.code == KeyCode::Char('?') {
                    if app.view == AppView::Help {
                        app.view = app.previous_view.take().unwrap_or(AppView::Library);
                    } else {
                        app.previous_view = Some(app.view);
                        app.view = AppView::Help;
                    }
                    continue;
                }

                match app.view {
                    AppView::Help => {
                        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                            app.view = app.previous_view.take().unwrap_or(AppView::Library);
                        }
                    }
                    AppView::Library => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('p') => {
                            // Cycle image protocols to debug cover rendering across terminals.
                            let next = app.image_picker.protocol_type().next();
                            app.image_picker.set_protocol_type(next);
                            app.load_selected_book_preview().ok();
                            app.refresh_current_book_render_cache().ok();
                        }
                        KeyCode::Char('n') => {
                            app.explorer_path = dirs::home_dir()
                                .unwrap_or_else(|| ".".into())
                                .to_string_lossy()
                                .to_string();
                            app.view = AppView::PathInput;
                        }
                        KeyCode::Char('S') => {
                            app.global_search_query.clear();
                            app.global_search_results.clear();
                            app.view = AppView::GlobalSearch;
                        }
                        KeyCode::Char('i') => {
                            app.view = AppView::Stats;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.books.is_empty() {
                                app.selected_book_index =
                                    (app.selected_book_index + 1) % app.books.len();
                                app.load_selected_book_preview().ok();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.books.is_empty() {
                                if app.selected_book_index > 0 {
                                    app.selected_book_index -= 1;
                                } else {
                                    app.selected_book_index = app.books.len() - 1;
                                }
                                app.load_selected_book_preview().ok();
                            }
                        }
                        KeyCode::Enter => {
                            let _ = app.open_selected_book();
                        }
                        _ => {}
                    },
                    AppView::Stats => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.view = AppView::Library,
                        _ => {}
                    },
                    AppView::PathInput => match key.code {
                        KeyCode::Esc => app.view = AppView::Library,
                        KeyCode::Enter => {
                            app.view = AppView::FileExplorer;
                            app.is_scanning = true;
                            app.explorer_results.clear();
                            let p = app.explorer_path.clone();
                            let tx = tx_scan.clone();
                            tokio::spawn(async move {
                                let res = App::scan_for_books_sync(p);
                                let _ = tx.send(res).await;
                            });
                        }
                        KeyCode::Char(c) => app.explorer_path.push(c),
                        KeyCode::Backspace => {
                            app.explorer_path.pop();
                        }
                        _ => {}
                    },
                    AppView::FileExplorer => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            if !app.is_scanning {
                                app.view = AppView::Library;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.explorer_results.is_empty() {
                                app.selected_explorer_index =
                                    (app.selected_explorer_index + 1) % app.explorer_results.len();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.explorer_results.is_empty() {
                                if app.selected_explorer_index > 0 {
                                    app.selected_explorer_index -= 1;
                                } else {
                                    app.selected_explorer_index = app.explorer_results.len() - 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if !app.is_scanning {
                                if let Some(path) =
                                    app.explorer_results.get(app.selected_explorer_index)
                                {
                                    let p_str = path.to_string_lossy().to_string();
                                    add_book_to_db(&mut app, &p_str).ok();
                                    app.refresh_library().ok();
                                    app.view = AppView::Library;
                                }
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
                                        let content = book
                                            .parser
                                            .get_chapter_content(chapter)
                                            .unwrap_or_default();

                                        let (chapter_content, image_protocols) =
                                            App::flatten_content(&mut app.image_picker, content);
                                        book.chapter_content = chapter_content;
                                        book.image_protocols = image_protocols;
                                    }
                                }
                            } else {
                                let q = app.global_search_query.clone();
                                if let Ok(results) = app.global_search(&q) {
                                    app.global_search_results = results;
                                    app.selected_search_index = 0;
                                }
                            }
                        }
                        KeyCode::Char(c) => app.global_search_query.push(c),
                        KeyCode::Backspace => {
                            app.global_search_query.pop();
                        }
                        _ => {}
                    },
                    AppView::Reader => match key.code {
                        KeyCode::Char('q') => {
                            app.save_progress().ok();
                            app.view = AppView::Library;
                            app.refresh_library().ok();
                        }
                        KeyCode::Char('s') => app.view = AppView::Select,
                        KeyCode::Char('A') => {
                            let _ = app.load_annotations();
                        }
                        KeyCode::Char('V') => {
                            let _ = app.load_vocabulary();
                        }
                        KeyCode::Char('E') => {
                            let _ = app.export_annotations();
                        }
                        KeyCode::Char('t') => app.open_toc(),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_viewport_down(),
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_viewport_up(),
                        KeyCode::Right | KeyCode::Char('l') => {
                            let _ = app.next_chapter();
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            let _ = app.prev_chapter();
                        }
                        KeyCode::Char('c') => app.toggle_theme(),
                        KeyCode::Char('[') | KeyCode::Char('-') => app.adjust_margin(1),
                        KeyCode::Char(']') | KeyCode::Char('+') | KeyCode::Char('=') => {
                            app.adjust_margin(-1)
                        }
                        KeyCode::Char('{') => app.adjust_spacing(1),
                        KeyCode::Char('}') => app.adjust_spacing(-1),
                        KeyCode::Char('/') => {
                            app.view = AppView::Search;
                            app.search_query.clear();
                        }
                        KeyCode::Char('a') => {
                            app.auto_scroll_active = !app.auto_scroll_active;
                            app.auto_scroll_last_tick = std::time::Instant::now();
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
                                if let Some(app::RenderLine::Text(line)) =
                                    book.chapter_content.get(book.current_line)
                                {
                                    if let Some(word) = line.split_whitespace().nth(book.word_index)
                                    {
                                        let clean_word: String =
                                            word.chars().filter(|c| c.is_alphabetic()).collect();
                                        if !clean_word.is_empty() {
                                            app.dictionary_query = clean_word.clone();
                                            app.view = AppView::Dictionary;
                                            app.dictionary_result = "Loading...".into();
                                            let tx_clone = tx_dict.clone();
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
                                app.selected_toc_index =
                                    (app.selected_toc_index + 1) % app.toc_items.len();
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
                    AppView::Annotation => match key.code {
                        KeyCode::Enter => {
                            let _ = app.add_annotation_with_note();
                        }
                        KeyCode::Esc => app.view = AppView::Select,
                        KeyCode::Char(c) => app.annotation_note.push(c),
                        KeyCode::Backspace => {
                            app.annotation_note.pop();
                        }
                        _ => {}
                    },
                    AppView::AnnotationList => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.view = AppView::Reader,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !app.current_annotations.is_empty() {
                                app.selected_annotation_index = (app.selected_annotation_index + 1)
                                    % app.current_annotations.len();
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.current_annotations.is_empty() {
                                if app.selected_annotation_index > 0 {
                                    app.selected_annotation_index -= 1;
                                } else {
                                    app.selected_annotation_index =
                                        app.current_annotations.len() - 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let _ = app.jump_to_annotation();
                        }
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
                                app.selected_vocab_index =
                                    (app.selected_vocab_index + 1) % app.vocabulary.len();
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
                                if let Some(pos) = book
                                    .chapter_content
                                    .iter()
                                    .skip(book.current_line + 1)
                                    .position(|l| {
                                        if let app::RenderLine::Text(text) = l {
                                            text.contains(&app.search_query)
                                        } else {
                                            false
                                        }
                                    })
                                {
                                    for _ in 0..(pos + 1) {
                                        app.move_cursor_down(viewport_height);
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
                    AppView::Rsvp => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.rsvp_active = false;
                            app.view = AppView::Reader;
                        }
                        KeyCode::Char(' ') => {
                            app.rsvp_active = !app.rsvp_active;
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => app.rsvp_wpm += 50,
                        KeyCode::Char('-') => {
                            if app.rsvp_wpm > 50 {
                                app.rsvp_wpm -= 50;
                            }
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
