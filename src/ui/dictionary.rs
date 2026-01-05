use crate::app::{App, Theme};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
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
        .margin(2)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.area());

    let title = Paragraph::new(format!(" Definition: {} ", app.dictionary_query)).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(fg).bg(bg)),
    );
    f.render_widget(title, chunks[0]);

    // Simple display of the result. For real "Premium" we'd parse the JSON properly.
    let display_text = if app.dictionary_result.starts_with('[') {
        // Try to show something readable if it's JSON from Dictionary API
        app.dictionary_result.chars().take(2000).collect::<String>()
    } else {
        app.dictionary_result.clone()
    };

    let content = Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Result (Esc to back) ")
                .style(Style::default().fg(fg).bg(bg)),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(content, chunks[1]);
}
