use crate::app::{App, Theme};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

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
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
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
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
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

    // Book Info & Progress Bar
    if let Some(selected_book) = app.books.get(app.selected_book_index) {
        let info_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(main_chunks[1]);

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
            .style(Style::default().fg(fg).bg(bg));
        f.render_widget(info_p, info_chunks[0]);

        let progress = if selected_book.total_lines > 0 {
            selected_book.lines_read as f64 / selected_book.total_lines as f64
        } else {
            0.0
        };
        let gauge = Gauge::default()
            .block(Block::default().title(" Progress ").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
            .ratio(progress);
        f.render_widget(gauge, info_chunks[1]);
    }

    let help = Paragraph::new(" [Enter] Open | [n] Add New | [S] Search | [?] Help | [q] Quit ")
        .style(Style::default().fg(fg).bg(bg));
    f.render_widget(help, chunks[2]);
}
