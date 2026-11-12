//! Dialogs over the panes: adding a feed (a URL box, then the feeds it
//! leads to, with folders to put the chosen one in), renaming, moving and
//! removing feeds and folders, and the open article's links.

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
    Finding { url: String },
    /// The feeds a URL leads to, even just one, to pick from and subscribe
    /// to, or first to add to folders in `folders`, an overlay.
    Choose {
        choices: Vec<FeedChoice>,
        list: ListState,
        focus: Button,
        /// Every folder, for the overlay.
        folder_names: Vec<String>,
        folders: Option<FolderPicker>,
    },
    /// Subscribing to `title`, then adding it to folders.
    Subscribing { title: String },
    /// The open article's links, as (text, URL).
    Links {
        links: Vec<(String, String)>,
        list: ListState,
    },
    /// Typing a new name for a feed or folder.
    Rename { target: Target, input: String },
    /// Asking before removing a feed or folder called `name`.
    Remove { target: Target, name: String },
    /// The folders a feed is in, checked, to change: the same picker as
    /// the feed selector's Add to Folder….
    Move {
        feed_id: String,
        picker: FolderPicker,
    },
}

/// The feed selector's buttons, beneath its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    AddToFolder,
    Subscribe,
    Move,
}

/// Folders to check for a new feed, over the feed selector; the row after
/// the last folder names a new one.
pub struct FolderPicker {
    pub folders: Vec<(String, bool)>,
    pub list: ListState,
    /// The name being typed for a new folder.
    pub new_folder: Option<String>,
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
    Find(String),
    Subscribe {
        url: String,
        title: Option<String>,
        folders: Vec<String>,
    },
    Open(String),
    Rename(Target, String),
    Remove(Target),
    /// Puts a feed in exactly these folders.
    Move {
        feed_id: String,
        folders: Vec<String>,
    },
}

impl Dialog {
    pub fn add() -> Self {
        Dialog::Add {
            input: String::new(),
            message: None,
        }
    }

    /// The feed selector, with `folder_names` to offer in its overlay.
    pub fn choose(choices: Vec<FeedChoice>, folder_names: Vec<String>) -> Self {
        Dialog::Choose {
            choices,
            list: ListState::default().with_selected(Some(0)),
            focus: Button::Subscribe,
            folder_names,
            folders: None,
        }
    }

    /// Presses a button of the feed selector, or of its folder overlay
    /// (which has only Subscribe).
    pub fn press(&mut self, button: Button) -> Action {
        if let Dialog::Move { feed_id, picker } = self {
            return match button {
                Button::Move => Action::Move {
                    feed_id: feed_id.clone(),
                    folders: picker.checked(),
                },
                Button::AddToFolder | Button::Subscribe => Action::None,
            };
        }
        let Dialog::Choose {
            choices,
            list,
            folder_names,
            folders,
            ..
        } = self
        else {
            return Action::None;
        };
        let Some(choice) = list.selected().and_then(|row| choices.get(row)) else {
            return Action::None;
        };
        let subscribe = |folders: Vec<String>| Action::Subscribe {
            url: choice.url.clone(),
            title: choice.title.clone(),
            folders,
        };
        match (button, folders.as_ref()) {
            (Button::Subscribe, Some(picker)) => subscribe(picker.checked()),
            (Button::Subscribe, None) => subscribe(Vec::new()),
            (Button::AddToFolder, _) => {
                *folders = Some(FolderPicker::new(folder_names.clone(), &[]));
                Action::None
            }
            (Button::Move, _) => Action::None,
        }
    }

    pub fn links(links: Vec<(String, String)>) -> Self {
        Dialog::Links {
            links,
            list: ListState::default().with_selected(Some(0)),
        }
    }

    /// Moving a feed now in `current`, choosing from every folder.
    pub fn move_to(feed_id: String, folders: Vec<String>, current: &[String]) -> Self {
        Dialog::Move {
            feed_id,
            picker: FolderPicker::new(folders, current),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        let in_folders = matches!(
            self,
            Dialog::Choose {
                folders: Some(_),
                ..
            } | Dialog::Move { .. }
        );
        if key.code == KeyCode::Esc && !in_folders {
            return Action::Close;
        }
        match self {
            Dialog::Add { input, message } => match key.code {
                KeyCode::Enter if !input.trim().is_empty() => {
                    Action::Find(input.trim().to_string())
                }
                _ => {
                    if edit(input, key) {
                        *message = None;
                    }
                    Action::None
                }
            },
            Dialog::Finding { .. } | Dialog::Subscribing { .. } => Action::None,
            Dialog::Choose {
                folders: Some(picker),
                ..
            } => match picker.on_key(key) {
                FolderKey::None => Action::None,
                FolderKey::Back => {
                    self.close_folders();
                    Action::None
                }
                FolderKey::Confirm => self.press(Button::Subscribe),
            },
            Dialog::Choose {
                choices,
                list,
                focus,
                ..
            } => match key.code {
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right => {
                    *focus = match focus {
                        Button::Subscribe => Button::AddToFolder,
                        Button::AddToFolder | Button::Move => Button::Subscribe,
                    };
                    Action::None
                }
                KeyCode::Enter => {
                    let button = *focus;
                    self.press(button)
                }
                _ => {
                    pick(key, list, choices.len());
                    Action::None
                }
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
            Dialog::Move { picker, .. } => match picker.on_key(key) {
                FolderKey::None => Action::None,
                FolderKey::Back => Action::Close,
                FolderKey::Confirm => self.press(Button::Move),
            },
        }
    }
}

impl Dialog {
    fn close_folders(&mut self) {
        if let Dialog::Choose { folders, .. } = self {
            *folders = None;
        }
    }
}

/// What a key in the folder overlay asks for.
enum FolderKey {
    None,
    /// Out of the picker: back to the feed selector, or closed.
    Back,
    /// Subscribe, or Move.
    Confirm,
}

impl FolderPicker {
    /// Every folder in `names`, with those in `checked` checked.
    fn new(names: Vec<String>, checked: &[String]) -> Self {
        FolderPicker {
            folders: names
                .into_iter()
                .map(|name| {
                    let is_checked = checked.contains(&name);
                    (name, is_checked)
                })
                .collect(),
            list: ListState::default().with_selected(Some(0)),
            new_folder: None,
        }
    }

    /// The checked folders' names.
    pub fn checked(&self) -> Vec<String> {
        self.folders
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Space checks a folder or starts naming a new one; Enter confirms,
    /// or, while naming, adds the new folder checked.
    fn on_key(&mut self, key: KeyEvent) -> FolderKey {
        if let Some(name) = &mut self.new_folder {
            match key.code {
                KeyCode::Esc => self.new_folder = None,
                KeyCode::Enter => {
                    let name = name.trim().to_string();
                    self.new_folder = None;
                    if !name.is_empty() {
                        match self.folders.iter_mut().find(|(folder, _)| *folder == name) {
                            Some((_, checked)) => *checked = true,
                            None => self.folders.push((name, true)),
                        }
                    }
                    self.list.select(Some(self.folders.len()));
                }
                _ => {
                    edit(name, key);
                }
            }
            return FolderKey::None;
        }
        match key.code {
            KeyCode::Esc => FolderKey::Back,
            KeyCode::Enter => FolderKey::Confirm,
            KeyCode::Char(' ') => {
                match self.list.selected() {
                    Some(row) if row < self.folders.len() => {
                        let checked = &mut self.folders[row].1;
                        *checked = !*checked;
                    }
                    _ => self.new_folder = Some(String::new()),
                }
                FolderKey::None
            }
            _ => {
                pick(key, &mut self.list, self.folders.len() + 1);
                FolderKey::None
            }
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
