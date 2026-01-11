use crate::app::{App, Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
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
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(f.area());

    if let Some(word) = app.rsvp_words.get(app.rsvp_index) {
        let p = Paragraph::new(word.as_str())
            .style(Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        f.render_widget(p, chunks[1]);
    }

    let help = Paragraph::new(format!(
        " WPM: {} | [Space] Pause | [+/-] Speed | [q] Back ",
        app.rsvp_wpm
    ))
    .alignment(Alignment::Center)
    .style(Style::default().fg(fg).bg(bg));
    f.render_widget(help, chunks[2]);
}
