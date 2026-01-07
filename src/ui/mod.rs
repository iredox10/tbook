pub mod annotation;
pub mod dictionary;
pub mod explorer;
pub mod globalsearch;
pub mod help;
pub mod library;
pub mod path_input;
pub mod reader;
pub mod rsvp;
pub mod toc;
pub mod vocabulary;

use crate::app::{App, AppView};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &mut App) {
    match app.view {
        AppView::Library => library::render(f, app),
        AppView::Reader | AppView::Search | AppView::Visual | AppView::Select => {
            reader::render(f, app)
        }
        AppView::Toc => toc::render(f, app),
        AppView::Rsvp => rsvp::render(f, app),
        AppView::Annotation => annotation::render_add(f, app),
        AppView::AnnotationList => annotation::render_list(f, app),
        AppView::Dictionary => dictionary::render(f, app),
        AppView::Vocabulary => vocabulary::render(f, app),
        AppView::GlobalSearch => globalsearch::render(f, app),
        AppView::PathInput => path_input::render(f, app),
        AppView::FileExplorer => explorer::render(f, app),
        AppView::Help => {
            help::render(f, app);
        }
    }

    if app.view == AppView::Help {
        help::render(f, app);
    }
}
