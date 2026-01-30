use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
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
        "i : View Reading Statistics",
        "n : Scan Drive for Books",
        "S : Global Search",
        "--- READER ---",
        "j/k : Scroll View",
        "a : Toggle Auto-Scroll",
        "+/- : Adjust Text Size (Zoom)",
        "f : Toggle Focus Mode",
        "p : Pomodoro Start/Pause",
        "R : Pomodoro Reset",
        "B : Skip Break",
        "s : Enter Select Mode",
        "t : Table of Contents",
        "A : View All Notes",
        "V : View Vocabulary",
        "E : Export to Markdown",
        "--- NOTES LIST ---",
        "1/2/3/4 : Filter Notes",
        "--- SELECT MODE ---",
        "j/k : Move Cursor",
        "w/b : Move by Word",
        "v : Start Visual Selection",
        "h : Highlight",
        "q : Question Highlight",
        "m : Summary Highlight",
        "d : Dictionary Lookup",
        "--- VISUAL MODE ---",
        "h : Highlight",
        "q : Question Highlight",
        "m : Summary Highlight",
        "a : Highlight + Note",
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
