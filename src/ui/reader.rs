use crate::app::{AnnotationKind, App, AppView, RenderLine, Theme};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{protocol::StatefulProtocol, StatefulImage};
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

fn wrap_words_to_lines<'a>(words: &'a [&'a str], max_width: u16) -> Vec<Vec<(usize, &'a str)>> {
    let max_width = max_width as usize;
    if max_width == 0 {
        return vec![Vec::new()];
    }

    let mut out: Vec<Vec<(usize, &str)>> = Vec::new();
    let mut current: Vec<(usize, &str)> = Vec::new();
    let mut current_w = 0usize;

    for (idx, w) in words.iter().enumerate() {
        let ww = UnicodeWidthStr::width(*w);
        let add_space = if current.is_empty() { 0 } else { 1 };
        if !current.is_empty() && current_w + add_space + ww > max_width {
            out.push(current);
            current = Vec::new();
            current_w = 0;
        }
        if !current.is_empty() {
            current_w += 1;
        }
        current.push((idx, *w));
        current_w += ww;
    }

    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(Vec::new());
    }
    out
}

pub fn render(f: &mut Frame, app: &mut App) {
    // Call these before mutably borrowing book
    let selection = app.get_selection_range();
    let (_, wpm) = app.get_reading_stats();
    let pomodoro_label = app.pomodoro_label();
    let pomodoro_running = app.pomodoro.running;
    let focus_mode = app.focus_mode;
    let view = app.view;
    let margin = app.margin;
    let line_spacing = app.line_spacing;

    if let Some(ref mut book) = app.current_book {
        let (bg, fg) = match app.theme {
            Theme::Default => (Color::Reset, Color::Reset),
            Theme::Gruvbox => (Color::Rgb(40, 40, 40), Color::Rgb(235, 219, 178)),
            Theme::Nord => (Color::Rgb(46, 52, 64), Color::Rgb(216, 222, 233)),
            Theme::Sepia => (Color::Rgb(250, 240, 230), Color::Rgb(93, 71, 139)),
        };

        let is_search = matches!(view, crate::app::AppView::Search);
        let show_top = !focus_mode;
        let show_status = !focus_mode || pomodoro_running;

        let constraints = [
            Constraint::Length(if show_top { 1 } else { 0 }),
            Constraint::Min(0),
            Constraint::Length(if is_search { 3 } else { 0 }),
            Constraint::Length(if show_status { 1 } else { 0 }),
        ];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(f.area());

        // Fill background
        f.render_widget(Block::default().style(Style::default().bg(bg)), f.area());

        // 1. Render Top Bar with Buttons
        if show_top {
            let top_bar_style = Style::default().bg(Color::Rgb(50, 50, 50)).fg(Color::White);
            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(15), // Buttons area
                ])
                .split(chunks[0]);

            let title_text = format!(" Reading: {}", book.path);
            f.render_widget(
                Paragraph::new(title_text).style(top_bar_style),
                top_chunks[0],
            );

            // Buttons for mouse click detection
            let buttons = Line::from(vec![
                Span::styled(
                    " [ - ] ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " [ + ] ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]);
            f.render_widget(Paragraph::new(buttons).style(top_bar_style), top_chunks[1]);
        }

        let _viewport_height = chunks[1].height as usize;
        let area = Layout::default()
            .margin(margin)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[1])[0];

        let mut rendered_protocols = HashSet::new();

        // When the terminal is narrow, render each paragraph as wrapped visual lines.
        // This avoids hard-cutting long lines and keeps text responsive.
        // We intentionally disable wrapping in Select/Visual so word indices map 1:1.
        let wrap_text = matches!(view, AppView::Reader | AppView::Search | AppView::Rsvp);

        let annotation_bg = |kind: &str| match AnnotationKind::from_str(kind) {
            AnnotationKind::Highlight => Color::Rgb(80, 60, 40),
            AnnotationKind::Question => Color::Rgb(40, 60, 120),
            AnnotationKind::Summary => Color::Rgb(40, 80, 40),
        };

        let mut y = area.y;
        let mut logical_i = book.viewport_top;
        while y < area.y.saturating_add(area.height) && logical_i < book.chapter_content.len() {
            let line_content = &book.chapter_content[logical_i];

            match line_content {
                RenderLine::Text(text) => {
                    if !wrap_text {
                        let line_area = Rect {
                            x: area.x,
                            y,
                            width: area.width,
                            height: 1,
                        };

                        let mut spans = Vec::new();
                        let words: Vec<&str> = text.split_whitespace().collect();

                        if words.is_empty() {
                            let mut style = Style::default().fg(fg).bg(bg);
                            if let Some((sl, _, el, _)) = selection {
                                if logical_i > sl && logical_i < el {
                                    style = style.bg(Color::Rgb(60, 60, 100));
                                }
                            }
                            f.render_widget(
                                Paragraph::new(Line::from(Span::styled(" ", style))),
                                line_area,
                            );
                            y = y.saturating_add(1);
                            logical_i += 1;
                            continue;
                        }

                        for (wi, word) in words.iter().enumerate() {
                            let mut style = Style::default().fg(fg).bg(bg);

                            for anno in &book.chapter_annotations {
                                let is_in_anno = if logical_i > anno.start_line
                                    && logical_i < anno.end_line
                                {
                                    true
                                } else if logical_i == anno.start_line && logical_i == anno.end_line
                                {
                                    wi >= anno.start_word && wi <= anno.end_word
                                } else if logical_i == anno.start_line {
                                    wi >= anno.start_word
                                } else if logical_i == anno.end_line {
                                    wi <= anno.end_word
                                } else {
                                    false
                                };

                                if is_in_anno {
                                    style = style.bg(annotation_bg(&anno.kind));
                                    break;
                                }
                            }

                            // Active selection highlight
                            let is_selected = if let Some((sl, sw, el, ew)) = selection {
                                if logical_i > sl && logical_i < el {
                                    true
                                } else if logical_i == sl && logical_i == el {
                                    wi >= sw && wi <= ew
                                } else if logical_i == sl {
                                    wi >= sw
                                } else if logical_i == el {
                                    wi <= ew
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            if is_selected {
                                style = style.bg(Color::Rgb(60, 60, 120)).fg(Color::White);
                            }

                            // Cursor highlight (Select/Visual)
                            if (app.view == AppView::Select || app.view == AppView::Visual)
                                && logical_i == book.current_line
                                && wi == book.word_index
                            {
                                style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                                if app.view == AppView::Visual {
                                    style = style.add_modifier(Modifier::UNDERLINED);
                                }
                            }

                            spans.push(Span::styled(format!("{} ", word), style));
                        }

                        f.render_widget(
                            Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false }),
                            line_area,
                        );
                        y = y.saturating_add(1);
                        logical_i += 1;
                        continue;
                    }

                    // Wrapped render path (Reader/Search): split into visual lines based on area.width
                    let words: Vec<&str> = text.split_whitespace().collect();
                    let wrapped = wrap_words_to_lines(&words, area.width);
                    for line_words in wrapped {
                        if y >= area.y.saturating_add(area.height) {
                            break;
                        }
                        let line_area = Rect {
                            x: area.x,
                            y,
                            width: area.width,
                            height: 1,
                        };

                        let mut spans = Vec::new();
                        for (wi, w) in line_words {
                            let mut style = Style::default().fg(fg).bg(bg);

                            // Persistent chapter highlights/annotations
                            for anno in &book.chapter_annotations {
                                let is_in_anno = if logical_i > anno.start_line
                                    && logical_i < anno.end_line
                                {
                                    true
                                } else if logical_i == anno.start_line && logical_i == anno.end_line
                                {
                                    wi >= anno.start_word && wi <= anno.end_word
                                } else if logical_i == anno.start_line {
                                    wi >= anno.start_word
                                } else if logical_i == anno.end_line {
                                    wi <= anno.end_word
                                } else {
                                    false
                                };

                                if is_in_anno {
                                    style = style.bg(annotation_bg(&anno.kind));
                                    break;
                                }
                            }

                            spans.push(Span::styled(format!("{} ", w), style));
                        }

                        f.render_widget(
                            Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false }),
                            line_area,
                        );
                        y = y.saturating_add(1 + line_spacing);
                    }

                    // We intentionally don't update selection/cursor highlighting here;
                    // Select/Visual already uses the non-wrapped path for correct indexing.
                    logical_i += 1;
                }
                RenderLine::Image {
                    protocol_idx,
                    row_idx,
                } => {
                    // Images already occupy multiple logical lines in chapter_content,
                    // so we can render them using the existing protocol logic.
                    let line_y = y;
                    if !rendered_protocols.contains(protocol_idx) {
                        rendered_protocols.insert(*protocol_idx);

                        let img_height_lines = book
                            .chapter_content
                            .iter()
                            .filter(|l| match l {
                                RenderLine::Image {
                                    protocol_idx: p, ..
                                } => p == protocol_idx,
                                _ => false,
                            })
                            .count();

                        let img_start_y = line_y as i32 - (*row_idx as i32);
                        let full_img_area = Rect {
                            x: area.x,
                            y: img_start_y.max(area.y as i32) as u16,
                            width: area.width,
                            height: img_height_lines as u16,
                        };

                        if let Some(protocol) = book.image_protocols.get_mut(*protocol_idx) {
                            let widget = StatefulImage::<StatefulProtocol>::default();
                            f.render_stateful_widget(widget, full_img_area, protocol);
                        }
                    }

                    // Advance one visual row; the remaining image rows will be visited as
                    // subsequent logical lines in the loop.
                    y = y.saturating_add(1);
                    logical_i += 1;
                }
            }
        }

        // Search Bar
        if is_search {
            let search = Paragraph::new(app.search_query.as_str()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Search (Regex supported) ")
                    .style(Style::default().fg(fg).bg(bg)),
            );
            f.render_widget(search, chunks[2]);
        }

        // Status bar
        if show_status {
            let mode_str = match view {
                AppView::Visual => " VISUAL ",
                AppView::Select => " SELECT ",
                _ => " NORMAL ",
            };
            let pomodoro = pomodoro_label.clone().unwrap_or_default();
            let status_text = if focus_mode {
                if pomodoro.is_empty() {
                    format!(
                        " FOCUS | Ch {} | L {} ",
                        book.current_chapter + 1,
                        book.current_line
                    )
                } else {
                    format!(
                        " FOCUS | {} | Ch {} | L {} ",
                        pomodoro,
                        book.current_chapter + 1,
                        book.current_line
                    )
                }
            } else {
                let pomodoro_section = if pomodoro.is_empty() {
                    String::new()
                } else {
                    format!(" | {}", pomodoro)
                };
                format!(
                    "{}| Ch: {}/{} | L: {} | WPM: {:.0}{} | 's' select | 't' toc | 'A' notes | 'q' lib ",
                    mode_str,
                    book.current_chapter + 1,
                    book.parser.get_chapter_count(),
                    book.current_line,
                    wpm,
                    pomodoro_section
                )
            };
            let status = Paragraph::new(status_text)
                .style(Style::default().bg(Color::Blue).fg(Color::White));
            f.render_widget(status, chunks[3]);
        }
    }
}
