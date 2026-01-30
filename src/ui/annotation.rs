use crate::app::{AnnotationKind, App, Theme};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render_add(f: &mut Frame, app: &mut App) {
    let (bg, fg) = get_theme_colors(app.theme);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(5)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());

    let help = Paragraph::new(" Type your note and press Enter to save, Esc to cancel ")
        .style(Style::default().fg(fg).bg(bg));
    f.render_widget(help, chunks[0]);

    let input = Paragraph::new(app.annotation_note.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Add Annotation/Note "),
        )
        .style(Style::default().fg(fg).bg(bg));
    f.render_widget(input, chunks[1]);
}

pub fn render_list(f: &mut Frame, app: &mut App) {
    let (bg, fg) = get_theme_colors(app.theme);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    let kind_color = |kind: &str| match AnnotationKind::from_str(kind) {
        AnnotationKind::Highlight => Color::Rgb(200, 170, 80),
        AnnotationKind::Question => Color::Rgb(120, 160, 220),
        AnnotationKind::Summary => Color::Rgb(140, 200, 140),
    };

    let items: Vec<ListItem> = app
        .current_annotations
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let style = if i == app.selected_annotation_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(kind_color(&a.kind)).bg(bg)
            };
            let note = a.note.as_deref().unwrap_or("No note");
            let kind = AnnotationKind::from_str(&a.kind).label();
            ListItem::new(format!(
                "{} Ch {}: {}... [{}]",
                kind,
                a.chapter + 1,
                &a.content[..std::cmp::min(20, a.content.len())],
                note
            ))
            .style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" Annotations ({}) ", app.annotation_filter.label()))
                .borders(Borders::ALL)
                .style(Style::default().fg(fg).bg(bg)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">> ");
    let mut list_state = ListState::default();
    if !app.current_annotations.is_empty() {
        list_state.select(Some(app.selected_annotation_index));
    }
    f.render_stateful_widget(list, chunks[0], &mut list_state);

    let footer = Paragraph::new(
        " [1] All | [2] Highlights | [3] Questions | [4] Summaries | [Enter] Jump | [Esc] Back ",
    )
    .style(Style::default().fg(fg).bg(bg));
    f.render_widget(footer, chunks[1]);
}

fn get_theme_colors(theme: Theme) -> (Color, Color) {
    match theme {
        Theme::Default => (Color::Reset, Color::Reset),
        Theme::Gruvbox => (Color::Rgb(40, 40, 40), Color::Rgb(235, 219, 178)),
        Theme::Nord => (Color::Rgb(46, 52, 64), Color::Rgb(216, 222, 233)),
        Theme::Sepia => (Color::Rgb(250, 240, 230), Color::Rgb(93, 71, 139)),
    }
}
