//! Dialogs over the panes: adding a feed (a URL box, then a list to pick
//! from when the URL leads to several feeds), renaming, moving and removing
//! feeds and folders, and the open article's links.

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
    /// Typing a new name for a feed or folder.
    Rename { target: Target, input: String },
    /// Asking before removing a feed or folder called `name`.
    Remove { target: Target, name: String },
    /// Picking the folder to move a feed to: `None` is no folder, and the
    /// row after the last folder makes a new one.
    Move {
        feed_id: String,
        folders: Vec<Option<String>>,
        list: ListState,
    },
    /// Typing the name of a new folder to move a feed to.
    NewFolder { feed_id: String, input: String },
}

/// What a dialog renames or removes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A feed, by id.
    Feed(String),
    /// A folder, by name.
    Folder(String),
}

/// What a key in a dialog asks the app to do.
pub enum Action {
    None,
    Close,
    Add(String),
    Open(String),
    Rename(Target, String),
    Remove(Target),
    Move {
        feed_id: String,
        folder: Option<String>,
    },
    NewFolder(String),
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

    /// Moving a feed now in `current`, choosing from `folders`.
    pub fn move_to(feed_id: String, folders: Vec<String>, current: Option<&str>) -> Self {
        let folders: Vec<Option<String>> = std::iter::once(None)
            .chain(folders.into_iter().map(Some))
            .collect();
        let selected = folders
            .iter()
            .position(|folder| folder.as_deref() == current)
            .unwrap_or(0);
        Dialog::Move {
            feed_id,
            folders,
            list: ListState::default().with_selected(Some(selected)),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        if key.code == KeyCode::Esc {
            return Action::Close;
        }
        match self {
            Dialog::Add { input, message } => match key.code {
                KeyCode::Enter if !input.trim().is_empty() => Action::Add(input.trim().to_string()),
                _ => {
                    if edit(input, key) {
                        *message = None;
                    }
                    Action::None
                }
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
            Dialog::Rename { target, input } => match key.code {
                KeyCode::Enter if !input.trim().is_empty() => {
                    Action::Rename(target.clone(), input.trim().to_string())
                }
                _ => {
                    edit(input, key);
                    Action::None
                }
            },
            Dialog::Remove { target, .. } => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => Action::Remove(target.clone()),
                KeyCode::Char('n') => Action::Close,
                _ => Action::None,
            },
            Dialog::Move {
                feed_id,
                folders,
                list,
            } => match pick(key, list, folders.len() + 1) {
                Some(row) if row == folders.len() => Action::NewFolder(feed_id.clone()),
                Some(row) => Action::Move {
                    feed_id: feed_id.clone(),
                    folder: folders[row].clone(),
                },
                None => Action::None,
            },
            Dialog::NewFolder { feed_id, input } => match key.code {
                KeyCode::Enter if !input.trim().is_empty() => Action::Move {
                    feed_id: feed_id.clone(),
                    folder: Some(input.trim().to_string()),
                },
                _ => {
                    edit(input, key);
                    Action::None
                }
            },
        }
    }
}

/// Types into a text box; returns whether the key changed it.
fn edit(input: &mut String, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Backspace => input.pop().is_some(),
        KeyCode::Char(c) => {
            input.push(c);
            true
        }
        _ => false,
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
