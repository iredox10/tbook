use crate::app::{App, AppView, Theme};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render(f: &mut Frame, app: &mut App) {
    if let Some(ref book) = app.current_book {
        let (bg, fg) = match app.theme {
            Theme::Default => (Color::Reset, Color::Reset),
            Theme::Gruvbox => (Color::Rgb(40, 40, 40), Color::Rgb(235, 219, 178)),
            Theme::Nord => (Color::Rgb(46, 52, 64), Color::Rgb(216, 222, 233)),
            Theme::Sepia => (Color::Rgb(250, 240, 230), Color::Rgb(93, 71, 139)),
        };

        let is_search = matches!(app.view, crate::app::AppView::Search);
        let constraints = if is_search {
            [
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ]
        } else {
            [
                Constraint::Min(0),
                Constraint::Length(0),
                Constraint::Length(1),
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(app.margin)
            .constraints(constraints.as_ref())
            .split(f.area());

        // Fill background
        f.render_widget(Block::default().style(Style::default().bg(bg)), f.area());

        let viewport_height = chunks[0].height as usize;

        // Selection range logic
        let selection = app.get_selection_range();

        // Process lines for rendering based on viewport
        let mut lines = Vec::new();
        for (i, line_content) in book
            .chapter_content
            .iter()
            .enumerate()
            .skip(book.viewport_top)
            .take(viewport_height)
        {
            let mut spans = Vec::new();
            let words: Vec<&str> = line_content.split_whitespace().collect();

            if words.is_empty() {
                let mut style = Style::default().fg(fg).bg(bg);
                if let Some((sl, _, el, _)) = selection {
                    if i > sl && i < el {
                        style = style.bg(Color::Rgb(60, 60, 100));
                    }
                }
                lines.push(Line::from(Span::styled(" ", style)));
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
            lines.push(Line::from(spans));

            // Add line spacing
            for _ in 0..app.line_spacing {
                lines.push(Line::from(""));
            }
        }

        let content = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: false });
        f.render_widget(content, chunks[0]);

        // Search Bar
        if is_search {
            let search = Paragraph::new(app.search_query.as_str()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Search (Regex supported) ")
                    .style(Style::default().fg(fg).bg(bg)),
            );
            f.render_widget(search, chunks[1]);
        }

        // Status bar
        let mode_str = match app.view {
            AppView::Visual => " VISUAL ",
            AppView::Select => " SELECT ",
            _ => " NORMAL ",
        };
        let (_, wpm) = app.get_reading_stats();
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
        f.render_widget(status, chunks[2]);
    }
}
