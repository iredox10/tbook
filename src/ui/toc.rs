use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(0)].as_ref())
        .split(f.area());

    let items: Vec<ListItem> = app
        .toc_items
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == app.selected_toc_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{:02}. {}", i + 1, t)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Table of Contents (Enter to Jump, Esc to Back) ")
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">> ");
    f.render_widget(list, chunks[0]);
}
