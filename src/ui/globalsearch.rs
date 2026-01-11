use crate::app::{App, Theme};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(bg)), f.area());

    let input = Paragraph::new(app.global_search_query.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Global Search (Type and press Enter) ")
            .style(Style::default().fg(fg).bg(bg)),
    );
    f.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .global_search_results
        .iter()
        .enumerate()
        .map(|(i, res)| {
            let style = if i == app.selected_search_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(fg).bg(bg)
            };
            ListItem::new(format!("{} [Ch {}]: {}", res.1, res.2 + 1, res.3)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Search Results ")
                .borders(Borders::ALL)
                .style(Style::default().fg(fg).bg(bg)),
        )
        .highlight_symbol(">> ");
    f.render_widget(list, chunks[1]);
}
