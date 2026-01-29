use crate::app::{App, Theme};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
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
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(bg)), f.area());

    let status_text = if app.is_scanning {
        format!(" Scanning: {} (Please wait...) ", app.explorer_path)
    } else {
        format!(" Scanning: {} ", app.explorer_path)
    };

    let title = Paragraph::new(status_text).block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(fg).bg(bg)),
    );
    f.render_widget(title, chunks[0]);

    let selected_count = app.explorer_selected.len();
    let total_count = app.explorer_results.len();

    if app.is_scanning {
        let loading = Paragraph::new(
            "\n\n\nSearching for books... This may take a moment depending on directory size.",
        )
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(fg).bg(bg));
        f.render_widget(loading, chunks[1]);
    } else {
        if app.explorer_results.is_empty() {
            let empty = Paragraph::new("\n\n\nNo books found for this path.")
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(fg).bg(bg));
            f.render_widget(empty, chunks[1]);
        } else {
            let items: Vec<ListItem> = app
                .explorer_results
                .iter()
                .enumerate()
                .map(|(i, path)| {
                    let style = if i == app.selected_explorer_index {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(fg).bg(bg)
                    };
                    let is_selected = app.explorer_selected.contains(path);
                    let checkbox = if is_selected { "[x]" } else { "[ ]" };
                    ListItem::new(format!("{} {}", checkbox, path.to_string_lossy())).style(style)
                })
                .collect();

            let list_title = format!(
                " Files Found ({}/{} selected) ",
                selected_count, total_count
            );
            let list = List::new(items)
                .block(
                    Block::default()
                        .title(list_title)
                        .borders(Borders::ALL)
                        .style(Style::default().fg(fg).bg(bg)),
                )
                .highlight_symbol(">> ");
            let mut list_state = ListState::default();
            if !app.explorer_results.is_empty() {
                list_state.select(Some(app.selected_explorer_index));
            }
            f.render_stateful_widget(list, chunks[1], &mut list_state);
        }
    }

    let footer = Paragraph::new(
        " [Space] Toggle | [a] All | [c] Clear | [Enter] Import | [i] Import All | [Esc] Back ",
    )
    .style(Style::default().fg(fg).bg(bg));
    f.render_widget(footer, chunks[2]);
}
