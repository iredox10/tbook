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
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, window_size,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui_image::picker::{Picker, ProtocolType};
use std::{io, time::{Duration, Instant}};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let config = AppConfig::load().unwrap_or_default();
    let mut app = App::new("tbook.db")?;
    app.apply_config(&config);

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
    app.image_picker = build_image_picker();

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

fn prefers_kitty_protocol() -> bool {
    let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default().to_lowercase();
    term.contains("kitty")
        || term.contains("ghostty")
        || term_program.contains("kitty")
        || term_program.contains("ghostty")
        || std::env::var("KITTY_WINDOW_ID").is_ok()
}

fn build_image_picker() -> Picker {
    let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

    if picker.protocol_type() == ProtocolType::Halfblocks {
        if let Ok(win) = window_size() {
            if win.columns > 0 && win.rows > 0 && win.width > 0 && win.height > 0 {
                let cell_width = win.width / win.columns;
                let cell_height = win.height / win.rows;
                if cell_width > 0 && cell_height > 0 {
                    #[allow(deprecated)]
                    let mut fallback = Picker::from_fontsize((cell_width, cell_height));
                    if prefers_kitty_protocol() {
                        fallback.set_protocol_type(ProtocolType::Kitty);
                    }
                    picker = fallback;
                }
            }
        }
    }

    picker
}

fn reader_content_height(
    term_height: u16,
    margin: u16,
    view: AppView,
    focus_mode: bool,
    show_status: bool,
) -> usize {
    let top_bar = if focus_mode { 0u16 } else { 1u16 };
    let status_bar = if show_status { 1u16 } else { 0u16 };
    let search_bar = if matches!(view, AppView::Search) { 3u16 } else { 0u16 };
    let content = term_height.saturating_sub(top_bar + status_bar + search_bar);
    let content = content.saturating_sub(margin.saturating_mul(2));
    content as usize
}

fn schedule_cover_request(
    app: &mut App,
    pending_cover_request: &mut Option<app::CoverRequest>,
    pending_cover_deadline: &mut Option<Instant>,
    delay: Duration,
) {
    *pending_cover_request = None;
    *pending_cover_deadline = None;
    if let Some(req) = app.cover_request_for_selected() {
        *pending_cover_request = Some(req);
        *pending_cover_deadline = Some(Instant::now() + delay);
    }
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
    let (tx_cover, mut rx_cover) = tokio::sync::mpsc::channel::<app::CoverResponse>(4);
    let (tx_cover_req, mut rx_cover_req) =
        tokio::sync::watch::channel::<Option<app::CoverRequest>>(None);

    let cover_debounce = Duration::from_millis(150);
    let mut pending_cover_request: Option<app::CoverRequest> = None;
    let mut pending_cover_deadline: Option<Instant> = None;

    let tx_cover_worker = tx_cover.clone();
    tokio::spawn(async move {
        while rx_cover_req.changed().await.is_ok() {
            let req = rx_cover_req.borrow().clone();
            let Some(req) = req else {
                continue;
            };
            let image = App::load_cover_image(&req.path);
            let _ = tx_cover_worker
                .send(app::CoverResponse {
                    book_id: req.book_id,
                    image,
                })
                .await;
        }
    });

    schedule_cover_request(
        &mut app,
        &mut pending_cover_request,
        &mut pending_cover_deadline,
        Duration::from_millis(0),
    );

    loop {
        let term_size = terminal
            .size()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let viewport_height = (term_size.height as usize).saturating_sub(1);
        let show_status = !app.focus_mode || app.pomodoro.running;
        let reader_height = reader_content_height(
            term_size.height,
            app.margin,
            app.view,
            app.focus_mode,
            show_status,
        )
        .max(1);

        terminal
            .draw(|f| ui::render(f, &mut app))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if let Ok(response) = rx_cover.try_recv() {
            app.apply_cover_response(response);
        }

        if let Ok(res) = rx_dict.try_recv() {
            app.dictionary_result = res.clone();
            app.db.add_to_vocabulary(&app.dictionary_query, &res).ok();
        }

        if let Ok(results) = rx_scan.try_recv() {
            app.explorer_results = results;
            app.is_scanning = false;
            app.selected_explorer_index = 0;
            app.explorer_selected.clear();
            if std::path::Path::new(&app.explorer_path).is_file()
                && app.explorer_results.len() == 1
            {
                app.explorer_selected
                    .insert(app.explorer_results[0].clone());
            }
        }

        app.tick_timers();

        // Auto-scroll logic
        if app.view == AppView::Reader && app.auto_scroll_active {
            if app.auto_scroll_last_tick.elapsed().as_millis() as u64 >= app.auto_scroll_interval_ms
            {
                app.scroll_viewport_down();
                app.auto_scroll_last_tick = std::time::Instant::now();
            }
        }

        if app.view == AppView::Library {
            if let (Some(req), Some(deadline)) = (&pending_cover_request, pending_cover_deadline) {
                if Instant::now() >= deadline {
                    app.mark_cover_request_in_flight(req.book_id);
                    let _ = tx_cover_req.send(Some(req.clone()));
                    pending_cover_request = None;
                    pending_cover_deadline = None;
                }
            }
        } else {
            pending_cover_request = None;
            pending_cover_deadline = None;
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
                        let next_view = app.previous_view.take().unwrap_or(AppView::Library);
                        app.view = next_view;
                        if app.view == AppView::Library {
                            schedule_cover_request(
                                &mut app,
                                &mut pending_cover_request,
                                &mut pending_cover_deadline,
                                Duration::from_millis(0),
                            );
                        }
                    } else {
                        app.previous_view = Some(app.view);
                        app.view = AppView::Help;
                    }
                    continue;
                }

                match app.view {
                    AppView::Help => {
                        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                            let next_view = app.previous_view.take().unwrap_or(AppView::Library);
                            app.view = next_view;
                            if app.view == AppView::Library {
                                schedule_cover_request(
                                    &mut app,
                                    &mut pending_cover_request,
                                    &mut pending_cover_deadline,
                                    Duration::from_millis(0),
                                );
                            }
                        }
                    }
                    AppView::Library => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('p') => {
                            // Cycle image protocols to debug cover rendering across terminals.
                            let next = app.image_picker.protocol_type().next();
                            app.image_picker.set_protocol_type(next);
                            app.refresh_current_book_render_cache().ok();
                            schedule_cover_request(
                                &mut app,
                                &mut pending_cover_request,
                                &mut pending_cover_deadline,
                                Duration::from_millis(0),
                            );
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
                                schedule_cover_request(
                                    &mut app,
                                    &mut pending_cover_request,
                                    &mut pending_cover_deadline,
                                    cover_debounce,
                                );
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !app.books.is_empty() {
                                if app.selected_book_index > 0 {
                                    app.selected_book_index -= 1;
                                } else {
                                    app.selected_book_index = app.books.len() - 1;
                                }
                                schedule_cover_request(
                                    &mut app,
                                    &mut pending_cover_request,
                                    &mut pending_cover_deadline,
                                    cover_debounce,
                                );
                            }
                        }
                        KeyCode::Enter => {
                            let _ = app.open_selected_book();
                        }
                        _ => {}
                    },
                    AppView::Stats => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.view = AppView::Library;
                            schedule_cover_request(
                                &mut app,
                                &mut pending_cover_request,
                                &mut pending_cover_deadline,
                                Duration::from_millis(0),
                            );
                        }
                        _ => {}
                    },
                    AppView::PathInput => match key.code {
                        KeyCode::Esc => {
                            app.view = AppView::Library;
                            schedule_cover_request(
                                &mut app,
                                &mut pending_cover_request,
                                &mut pending_cover_deadline,
                                Duration::from_millis(0),
                            );
                        }
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
                                schedule_cover_request(
                                    &mut app,
                                    &mut pending_cover_request,
                                    &mut pending_cover_deadline,
                                    Duration::from_millis(0),
                                );
                            }
                        }
                        KeyCode::Char(' ') => {
                            if !app.is_scanning {
                                app.toggle_explorer_selection();
                            }
                        }
                        KeyCode::Char('a') => {
                            if !app.is_scanning {
                                app.select_all_explorer_results();
                            }
                        }
                        KeyCode::Char('c') => {
                            if !app.is_scanning {
                                app.clear_explorer_selection();
                            }
                        }
                        KeyCode::Char('i') => {
                            if !app.is_scanning {
                                app.select_all_explorer_results();
                                let _ = app.import_explorer_selection();
                                app.refresh_library().ok();
                                app.view = AppView::Library;
                                schedule_cover_request(
                                    &mut app,
                                    &mut pending_cover_request,
                                    &mut pending_cover_deadline,
                                    Duration::from_millis(0),
                                );
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
                                if !app.explorer_results.is_empty() {
                                    let _ = app.import_explorer_selection();
                                    app.refresh_library().ok();
                                    app.view = AppView::Library;
                                    schedule_cover_request(
                                        &mut app,
                                        &mut pending_cover_request,
                                        &mut pending_cover_deadline,
                                        Duration::from_millis(0),
                                    );
                                }
                            }
                        }
                        _ => {}
                    },
                    AppView::GlobalSearch => match key.code {
                        KeyCode::Esc => {
                            app.view = AppView::Library;
                            schedule_cover_request(
                                &mut app,
                                &mut pending_cover_request,
                                &mut pending_cover_deadline,
                                Duration::from_millis(0),
                            );
                        }
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
                            schedule_cover_request(
                                &mut app,
                                &mut pending_cover_request,
                                &mut pending_cover_deadline,
                                Duration::from_millis(0),
                            );
                        }
                        KeyCode::Char('f') => app.toggle_focus_mode(),
                        KeyCode::Char('p') => app.pomodoro_toggle(),
                        KeyCode::Char('R') => app.pomodoro_reset(),
                        KeyCode::Char('B') => app.pomodoro_skip_break(),
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
                            if app.view == AppView::Visual || app.view == AppView::Select {
                                let _ = app.add_quick_highlight();
                            }
                        }
                        KeyCode::Char('q') => {
                            if app.view == AppView::Visual || app.view == AppView::Select {
                                let _ = app.add_question_highlight();
                            }
                        }
                        KeyCode::Char('m') => {
                            if app.view == AppView::Visual || app.view == AppView::Select {
                                let _ = app.add_summary_highlight();
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
                        KeyCode::Char('f') => app.toggle_focus_mode(),
                        KeyCode::Char('p') => app.pomodoro_toggle(),
                        KeyCode::Char('R') => app.pomodoro_reset(),
                        KeyCode::Char('B') => app.pomodoro_skip_break(),
                        KeyCode::Down | KeyCode::Char('j') => app.move_cursor_down(reader_height),
                        KeyCode::Up | KeyCode::Char('k') => app.move_cursor_up(),
                        KeyCode::Char('w') => app.cursor_right(reader_height),
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
                        KeyCode::Char('1') => app.set_annotation_filter(app::AnnotationFilter::All),
                        KeyCode::Char('2') => {
                            app.set_annotation_filter(app::AnnotationFilter::Highlight)
                        }
                        KeyCode::Char('3') => {
                            app.set_annotation_filter(app::AnnotationFilter::Question)
                        }
                        KeyCode::Char('4') => {
                            app.set_annotation_filter(app::AnnotationFilter::Summary)
                        }
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
