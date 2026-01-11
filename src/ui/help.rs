use crate::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(f: &mut Frame, _app: &App) {
    let area = centered_rect(60, 60, f.area());
    f.render_widget(Clear, area); // Clear the area for the popup

    let help_text = vec![
        "--- GLOBAL ---",
        "? : Toggle Help",
        "q : Back / Quit",
        "--- LIBRARY ---",
        "Enter : Open Book",
        "n : Scan Drive for Books",
        "S : Global Search",
        "--- READER ---",
        "j/k : Scroll View",
        "s : Enter Select Mode",
        "t : Table of Contents",
        "A : View All Notes",
        "V : View Vocabulary",
        "E : Export to Markdown",
        "--- SELECT MODE ---",
        "j/k : Move Cursor",
        "w/b : Move by Word",
        "v : Start Visual Selection",
        "d : Dictionary Lookup",
        "--- VISUAL MODE ---",
        "h : Quick Highlight (Gold)",
        "a : Highlight + Note (Green)",
    ];

    let p = Paragraph::new(help_text.join("\n"))
        .block(Block::default().title(" Quick Help ").borders(Borders::ALL))
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White).bg(Color::Black));
    f.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
