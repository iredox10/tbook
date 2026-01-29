use crate::app::{App, Theme};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
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
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(f.area());

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(bg)), f.area());

    // List of words
    let items: Vec<ListItem> = app
        .vocabulary
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let style = if i == app.selected_vocab_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(fg).bg(bg)
            };
            ListItem::new(format!("{} ({})", v.word, v.lookup_count)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Vocabulary List ")
                .borders(Borders::ALL)
                .style(Style::default().fg(fg).bg(bg)),
        )
        .highlight_symbol(">> ");
    f.render_widget(list, chunks[0]);

    // Definition display
    if let Some(vocab) = app.vocabulary.get(app.selected_vocab_index) {
        let def = Paragraph::new(vocab.definition.as_str())
            .block(
                Block::default()
                    .title(format!(" {} ", vocab.word))
                    .borders(Borders::ALL)
                    .style(Style::default().fg(fg).bg(bg)),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(def, chunks[1]);
    }
}
