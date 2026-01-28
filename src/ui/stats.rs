use crate::app::{App, Theme};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{BarChart, Block, Borders, Paragraph},
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
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(bg)), f.area());

    let title = Paragraph::new(" Reading Statistics (Last 7 Days) ")
        .block(Block::default().borders(Borders::ALL))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(title, chunks[0]);

    if let Ok(stats) = app.db.get_weekly_stats() {
        let data: Vec<(&str, u64)> = stats.iter().map(|(d, w)| (d.as_str(), *w as u64)).collect();

        let barchart = BarChart::default()
            .block(
                Block::default()
                    .title(" Words Read per Day ")
                    .borders(Borders::ALL),
            )
            .data(&data)
            .bar_width(12)
            .bar_gap(2)
            .bar_style(Style::default().fg(Color::Green))
            .value_style(Style::default().fg(Color::Black).bg(Color::Green));

        f.render_widget(barchart, chunks[1]);
    } else {
        let error = Paragraph::new("No statistics available yet. Start reading!")
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(error, chunks[1]);
    }

    let footer = Paragraph::new(" [q] Back to Library ").style(Style::default().fg(fg).bg(bg));
    f.render_widget(footer, chunks[2]);
}
