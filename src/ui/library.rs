use crate::app::{App, Theme};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{protocol::StatefulProtocol, FilterType, Resize, StatefulImage};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn render(f: &mut Frame, app: &mut App) {
    let (bg, fg) = match app.theme {
        Theme::Default => (Color::Reset, Color::Reset),
        Theme::Gruvbox => (Color::Rgb(40, 40, 40), Color::Rgb(235, 219, 178)),
        Theme::Nord => (Color::Rgb(46, 52, 64), Color::Rgb(216, 222, 233)),
        Theme::Sepia => (Color::Rgb(250, 240, 230), Color::Rgb(93, 71, 139)),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(bg)), f.area());

    let title = Paragraph::new(" TBook - Premium Terminal Reader ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(fg).bg(bg)),
        )
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(title, chunks[0]);

    // Split center area for list and a potential preview or info
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    let items: Vec<ListItem> = app
        .books
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let style = if i == app.selected_book_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(fg).bg(bg)
            };

            let progress = if b.total_lines > 0 {
                (b.lines_read as f64 / b.total_lines as f64) * 100.0
            } else {
                0.0
            };

            ListItem::new(format!("{:<30} | {:>3.0}%", b.title, progress)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Library ")
                .borders(Borders::ALL)
                .style(Style::default().fg(fg).bg(bg)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">> ");
    f.render_widget(list, main_chunks[0]);

    // Book Info & Cover Preview
    if let Some(selected_book) = app.books.get(app.selected_book_index) {
        let info_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(6),    // Cover area (keep visible on small terminals)
                Constraint::Length(8), // Text info area
                Constraint::Length(3), // Progress bar
            ])
            .split(main_chunks[1]);

        // 1. Render Cover
        let cover_block = Block::default()
            .title(" Preview ")
            .borders(Borders::ALL)
            .style(Style::default().fg(fg).bg(bg));
        let cover_inner = cover_block.inner(info_chunks[0]);
        f.render_widget(cover_block, info_chunks[0]);

        let selected_id = selected_book.id;
        let is_cover_loading = app.current_library_cover.is_none()
            && !app.cover_cache.contains_key(&selected_id)
            && !app.cover_missing.contains(&selected_id);

        if let Some(ref mut protocol) = app.current_library_cover {
            // Use a higher quality resize filter so downscaled covers look less muddy.
            let widget = StatefulImage::<StatefulProtocol>::default()
                .resize(Resize::Fit(Some(FilterType::Lanczos3)));
            f.render_stateful_widget(widget, cover_inner, protocol);
        } else if is_cover_loading {
            const SPINNER: [&str; 4] = ["-", "\\", "|", "/"];
            let ticks = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as usize)
                .unwrap_or(0);
            let spinner = SPINNER[(ticks / 120) % SPINNER.len()];
            let loading = Paragraph::new(format!("\n\n\nLoading cover {}", spinner))
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(loading, cover_inner);
        } else {
            let no_cover = Paragraph::new("\n\n\n[ No Cover Preview ]")
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(no_cover, cover_inner);
        }

        // 2. Render Text Info
        let info = format!(
            "Title: {}\nAuthor: {}\nPath: {}\nChapters: {}\nTotal Lines: {}",
            selected_book.title,
            selected_book.author,
            selected_book.path,
            selected_book.total_chapters,
            selected_book.total_lines
        );
        let info_p = Paragraph::new(info)
            .block(
                Block::default()
                    .title(" Book Info ")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(fg).bg(bg)),
            )
            .style(Style::default().fg(fg).bg(bg))
            .wrap(Wrap { trim: true });
        f.render_widget(info_p, info_chunks[1]);

        // 3. Render Progress Gauge
        let progress = if selected_book.total_lines > 0 {
            selected_book.lines_read as f64 / selected_book.total_lines as f64
        } else {
            0.0
        };
        let gauge = Gauge::default()
            .block(Block::default().title(" Progress ").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
            .ratio(progress);
        f.render_widget(gauge, info_chunks[2]);
    }

    let proto = format!(
        "proto={:?} font={:?}  ([p] cycle proto)",
        app.image_picker.protocol_type(),
        app.image_picker.font_size()
    );
    let help = Paragraph::new(format!(
        " [Enter] Open | [n] Add New | [S] Search | [?] Help | [p] Proto | [q] Quit  |  {} ",
        proto
    ))
    .style(Style::default().fg(fg).bg(bg));
    f.render_widget(help, chunks[2]);
}
