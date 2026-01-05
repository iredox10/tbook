use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.area());

    let title = Paragraph::new(" TBook - Premium Terminal Reader ")
        .block(Block::default().borders(Borders::ALL))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = app
        .books
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let style = if i == app.selected_book_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{} - {}", b.title, b.author)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(" Library ").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">> ");
    f.render_widget(list, chunks[1]);

    let help = Paragraph::new(" [Enter] Open Book | [j/k] Navigate | [q] Quit ");
    f.render_widget(help, chunks[2]);
}
