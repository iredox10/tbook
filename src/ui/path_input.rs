use crate::app::{App, Theme};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
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
        .margin(5)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(bg)), f.area());

    let title = Paragraph::new(" Enter Directory Path to Scan ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(fg).bg(bg)),
        )
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(title, chunks[0]);

    let input = Paragraph::new(app.explorer_path.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Path ")
                .style(Style::default().fg(fg).bg(bg)),
        )
        .alignment(Alignment::Left);
    f.render_widget(input, chunks[1]);

    let help = Paragraph::new(" [Enter] Start Scan | [Esc] Cancel ")
        .alignment(Alignment::Center)
        .style(Style::default().fg(fg).bg(bg));
    let help_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(chunks[1]);
    f.render_widget(help, help_chunks[1]);
}
