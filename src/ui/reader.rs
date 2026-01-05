use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    if let Some(ref book) = app.current_book {
        let constraints = if matches!(app.view, crate::app::AppView::Search) {
            [
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ]
        } else {
            [
                Constraint::Min(0),
                Constraint::Length(0),
                Constraint::Length(1),
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints(constraints.as_ref())
            .split(f.area());

        // Content
        let visible_lines: Vec<String> = book
            .chapter_content
            .iter()
            .skip(book.current_line)
            .cloned()
            .collect();

        let content = Paragraph::new(visible_lines.join("\n"))
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: false });
        f.render_widget(content, chunks[0]);

        // Search Bar
        if matches!(app.view, crate::app::AppView::Search) {
            let search = Paragraph::new(app.search_query.as_str()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Search (Regex supported) "),
            );
            f.render_widget(search, chunks[1]);
        }

        // Status bar
        let progress = format!(
            " Chapter: {}/{} | Line: {} | '/' search | 't' toc | 'q' library ",
            book.current_chapter + 1,
            book.parser.get_chapter_count(),
            book.current_line
        );
        let status =
            Paragraph::new(progress).style(Style::default().bg(Color::Blue).fg(Color::White));
        f.render_widget(status, chunks[2]);
    }
}
