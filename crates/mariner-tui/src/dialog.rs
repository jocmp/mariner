//! The add-feed dialog: a URL box, then a list to pick from when the URL
//! leads to several feeds.

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
}

/// What a key in the dialog asks the app to do.
pub enum Action {
    None,
    Close,
    Add(String),
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
            Dialog::Choose { choices, list } => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    let next = list
                        .selected()
                        .map_or(0, |row| (row + 1).min(choices.len() - 1));
                    list.select(Some(next));
                    Action::None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let previous = list.selected().map_or(0, |row| row.saturating_sub(1));
                    list.select(Some(previous));
                    Action::None
                }
                KeyCode::Enter => list
                    .selected()
                    .and_then(|row| choices.get(row))
                    .map_or(Action::None, |choice| Action::Add(choice.url.clone())),
                _ => Action::None,
            },
        }
    }
}
