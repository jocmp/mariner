//! Dialogs over the panes: adding a feed (a URL box, then a list to pick
//! from when the URL leads to several feeds), and the open article's links.

use mariner_core::FeedChoice;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

pub enum Dialog {
    /// Typing a URL; `message` explains why the last one didn't work.
    Add {
        input: String,
        message: Option<String>,
    },
    /// Finding the feeds at `url`.
    Adding { url: String },
    /// The URL leads to several feeds.
    Choose {
        choices: Vec<FeedChoice>,
        list: ListState,
    },
    /// The open article's links, as (text, URL).
    Links {
        links: Vec<(String, String)>,
        list: ListState,
    },
}

/// What a key in a dialog asks the app to do.
pub enum Action {
    None,
    Close,
    Add(String),
    Open(String),
}

impl Dialog {
    pub fn add() -> Self {
        Dialog::Add {
            input: String::new(),
            message: None,
        }
    }

    pub fn choose(choices: Vec<FeedChoice>) -> Self {
        Dialog::Choose {
            choices,
            list: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn links(links: Vec<(String, String)>) -> Self {
        Dialog::Links {
            links,
            list: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        if key.code == KeyCode::Esc {
            return Action::Close;
        }
        match self {
            Dialog::Add { input, message } => match key.code {
                KeyCode::Enter if !input.trim().is_empty() => Action::Add(input.trim().to_string()),
                KeyCode::Backspace => {
                    input.pop();
                    Action::None
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    *message = None;
                    Action::None
                }
                _ => Action::None,
            },
            Dialog::Adding { .. } => Action::None,
            Dialog::Choose { choices, list } => match pick(key, list, choices.len()) {
                Some(row) => Action::Add(choices[row].url.clone()),
                None => Action::None,
            },
            Dialog::Links { links, list } => match pick(key, list, links.len()) {
                Some(row) => Action::Open(links[row].1.clone()),
                None => Action::None,
            },
        }
    }
}

/// Moves through a list with j/k or the arrows; Enter picks the selected
/// row.
fn pick(key: KeyEvent, list: &mut ListState, rows: usize) -> Option<usize> {
    let last = rows.saturating_sub(1);
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            list.select(Some(list.selected().map_or(0, |row| (row + 1).min(last))));
            None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            list.select(Some(list.selected().map_or(0, |row| row.saturating_sub(1))));
            None
        }
        KeyCode::Enter => list.selected().filter(|row| *row < rows),
        _ => None,
    }
}
