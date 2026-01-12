use crate::app::{App, AppView, RenderLine, Theme};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use ratatui_image::StatefulImage;
use std::collections::HashSet;

pub fn render(f: &mut Frame, app: &mut App) {
    // Call these before mutably borrowing book
    let selection = app.get_selection_range();
    let (_, wpm) = app.get_reading_stats();

    if let Some(ref mut book) = app.current_book {
        let (bg, fg) = match app.theme {
            Theme::Default => (Color::Reset, Color::Reset),
            Theme::Gruvbox => (Color::Rgb(40, 40, 40), Color::Rgb(235, 219, 178)),
            Theme::Nord => (Color::Rgb(46, 52, 64), Color::Rgb(216, 222, 233)),
            Theme::Sepia => (Color::Rgb(250, 240, 230), Color::Rgb(93, 71, 139)),
        };

        let is_search = matches!(app.view, crate::app::AppView::Search);

        let constraints = if is_search {
            [
                Constraint::Length(1), // Top bar
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Search bar
                Constraint::Length(1), // Status bar
            ]
        } else {
            [
                Constraint::Length(1), // Top bar
                Constraint::Min(0),    // Content
                Constraint::Length(0), // No search
                Constraint::Length(1), // Status bar
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints.as_ref())
            .split(f.area());

        // Fill background
        f.render_widget(Block::default().style(Style::default().bg(bg)), f.area());

        // 1. Render Top Bar with Buttons
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

        let viewport_height = chunks[1].height as usize;
        let area = Layout::default()
            .margin(app.margin)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[1])[0];

        let mut rendered_protocols = HashSet::new();

        // Process lines for rendering based on viewport
        for (i, line_content) in book
            .chapter_content
            .iter()
            .enumerate()
            .skip(book.viewport_top)
            .take(viewport_height)
        {
            let line_y = area.y + (i - book.viewport_top) as u16;
            if line_y >= area.y + area.height {
                break;
            }

            let line_area = Rect {
                x: area.x,
                y: line_y,
                width: area.width,
                height: 1,
            };

            match line_content {
                RenderLine::Text(text) => {
                    let mut spans = Vec::new();
                    let words: Vec<&str> = text.split_whitespace().collect();

                    if words.is_empty() {
                        let mut style = Style::default().fg(fg).bg(bg);
                        if let Some((sl, _, el, _)) = selection {
                            if i > sl && i < el {
                                style = style.bg(Color::Rgb(60, 60, 100));
                            }
                        }
                        f.render_widget(
                            Paragraph::new(Line::from(Span::styled(" ", style))),
                            line_area,
                        );
                        continue;
                    }

                    for (wi, word) in words.iter().enumerate() {
                        let mut style = Style::default().fg(fg).bg(bg);

                        // 1. Check for persistent chapter highlights/annotations
                        for anno in &book.chapter_annotations {
                            let is_in_anno = if i > anno.start_line && i < anno.end_line {
                                true
                            } else if i == anno.start_line && i == anno.end_line {
                                wi >= anno.start_word && wi <= anno.end_word
                            } else if i == anno.start_line {
                                wi >= anno.start_word
                            } else if i == anno.end_line {
                                wi <= anno.end_word
                            } else {
                                false
                            };

                            if is_in_anno {
                                if anno.note.is_some() {
                                    style = style.bg(Color::Rgb(40, 80, 40));
                                } else {
                                    style = style.bg(Color::Rgb(80, 60, 40));
                                }
                                break;
                            }
                        }

                        // 2. Check for active selection highlight (Indigo)
                        let is_selected = if let Some((sl, sw, el, ew)) = selection {
                            if i > sl && i < el {
                                true
                            } else if i == sl && i == el {
                                wi >= sw && wi <= ew
                            } else if i == sl {
                                wi >= sw
                            } else if i == el {
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

                        // 3. Check for cursor (Only in Select/Visual mode)
                        if (app.view == AppView::Select || app.view == AppView::Visual)
                            && i == book.current_line
                            && wi == book.word_index
                        {
                            style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                            if app.view == AppView::Visual {
                                style = style.add_modifier(Modifier::UNDERLINED);
                            }
                        }

                        spans.push(Span::styled(format!("{} ", word), style));
                    }

                    let p = Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false });
                    f.render_widget(p, line_area);
                }
                RenderLine::Image {
                    protocol_idx,
                    row_idx,
                    orig_width: _,
                    orig_height: _,
                } => {
                    if !rendered_protocols.contains(protocol_idx) {
                        rendered_protocols.insert(*protocol_idx);

                        // Find how many lines this image occupies in the content
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
                            let widget = StatefulImage::new(None);
                            f.render_stateful_widget(widget, full_img_area, protocol);
                        }
                    }
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
        let mode_str = match app.view {
            AppView::Visual => " VISUAL ",
            AppView::Select => " SELECT ",
            _ => " NORMAL ",
        };
        let progress = format!(
            "{}| Ch: {}/{} | L: {} | WPM: {:.0} | 's' select | 't' toc | 'A' notes | 'q' lib ",
            mode_str,
            book.current_chapter + 1,
            book.parser.get_chapter_count(),
            book.current_line,
            wpm
        );
        let status =
            Paragraph::new(progress).style(Style::default().bg(Color::Blue).fg(Color::White));
        f.render_widget(status, chunks[3]);
    }
}
