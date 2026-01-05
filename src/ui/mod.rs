pub mod library;
pub mod reader;
pub mod toc;

use crate::app::{App, AppView};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &mut App) {
    match app.view {
        AppView::Library => library::render(f, app),
        AppView::Reader | AppView::Search => reader::render(f, app),
        AppView::Toc => toc::render(f, app),
    }
}
