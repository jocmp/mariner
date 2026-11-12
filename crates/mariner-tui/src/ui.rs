//! Drawing: three panes side by side when the terminal is wide enough,
//! otherwise only the focused one.

use mariner_core::ArticleFilter;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, Areas, Pane, Row};
use crate::dialog::{Button, Dialog, FolderPicker, Target};
use crate::reader;

/// Narrower terminals show one pane at a time.
const SIDE_BY_SIDE_WIDTH: u16 = 120;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [main, status] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
    if main.width >= SIDE_BY_SIDE_WIDTH {
        let [feeds, articles, reader] = Layout::horizontal([
            Constraint::Length(30),
            Constraint::Percentage(35),
            Constraint::Fill(1),
        ])
        .areas(main);
        app.areas = Areas {
            feeds: Some(feeds),
            articles: Some(articles),
            reader: Some(reader),
        };
        draw_feeds(frame, app, feeds);
        draw_articles(frame, app, articles);
        draw_reader(frame, app, reader);
    } else {
        app.areas = Areas::default();
        match app.focus {
            Pane::Feeds => {
                app.areas.feeds = Some(main);
                draw_feeds(frame, app, main);
            }
            Pane::Articles => {
                app.areas.articles = Some(main);
                draw_articles(frame, app, main);
            }
            Pane::Reader => {
                app.areas.reader = Some(main);
                draw_reader(frame, app, main);
            }
        }
    }
    draw_status(frame, app, status);
    app.buttons.clear();
    if let Some(dialog) = &mut app.dialog {
        draw_dialog(frame, dialog, &mut app.buttons);
    }
}

/// The dialog, centered over everything.
fn draw_dialog(frame: &mut Frame, dialog: &mut Dialog, buttons: &mut Vec<(Rect, Button)>) {
    let screen = frame.area();
    let width = screen.width.saturating_sub(4).min(72);
    let height = match dialog {
        // The list, a blank line and two buttons.
        Dialog::Choose { choices, .. } => (choices.len() as u16 + 7).min(screen.height),
        Dialog::Links { links, .. } => {
            (links.len() as u16 + 2).min(screen.height.saturating_sub(2))
        }
        Dialog::Move { picker, .. } => picker_height(picker).min(screen.height),
        _ => 6.min(screen.height),
    };
    let area = Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y + screen.height.saturating_sub(height) / 3,
        width,
        height,
    };
    frame.render_widget(Clear, area);
    match dialog {
        Dialog::Add { input, message } => {
            let mut lines = vec![input_line("URL: ", input), Line::default()];
            match message {
                Some(message) => lines.push(Line::from(message.clone()).red()),
                None => {
                    lines.push(Line::from("A site or feed URL; Enter to add, Esc to cancel").dim())
                }
            }
            let paragraph = Paragraph::new(lines)
                .block(pane("Add feed", true))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
        Dialog::Finding { url } => {
            let lines = vec![
                Line::from(format!("Finding feeds at {url}…")),
                Line::default(),
                Line::from("Esc to cancel").dim(),
            ];
            let paragraph = Paragraph::new(lines)
                .block(pane("Add feed", true))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
        Dialog::Subscribing { title } => {
            let lines = vec![
                Line::from(format!("Subscribing to {title}…")),
                Line::default(),
                Line::from("Esc to close; it still subscribes").dim(),
            ];
            let paragraph = Paragraph::new(lines)
                .block(pane("Add feed", true))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
        Dialog::Links { links, list } => {
            let items: Vec<ListItem> = links
                .iter()
                .map(|(text, url)| {
                    ListItem::new(Line::from(vec![
                        Span::from(text.clone()),
                        Span::from(format!("  {url}")).dim(),
                    ]))
                })
                .collect();
            let list_widget = List::new(items)
                .block(pane("Links", true).title_bottom(" Enter to open · Esc to close "))
                .highlight_style(highlight());
            frame.render_stateful_widget(list_widget, area, list);
        }
        Dialog::Rename { target, input } => {
            let title = match target {
                Target::Feed(_) => "Rename feed",
                Target::Folder(_) => "Rename folder",
            };
            let hint = "Enter to rename, Esc to cancel";
            frame.render_widget(text_box(title, "Name: ", input, hint), area);
        }
        Dialog::Remove { target, name } => {
            let (title, question) = match target {
                Target::Feed(_) => (
                    "Remove feed",
                    format!("Unsubscribe from {name}? Its articles go too, starred ones included."),
                ),
                Target::Folder(_) => (
                    "Remove folder",
                    format!("Remove the folder {name}? Its feeds stay."),
                ),
            };
            let lines = vec![
                Line::from(question),
                Line::default(),
                Line::from("y to remove, n or Esc to cancel").dim(),
            ];
            let paragraph = Paragraph::new(lines)
                .block(pane(title, true))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
        Dialog::Move { picker, .. } => {
            draw_folder_picker(frame, area, picker, "Move to", Button::Move, buttons);
        }
        Dialog::Choose {
            choices,
            list,
            focus,
            folders,
            ..
        } => {
            let block = pane("Add feed", true).title_bottom(" Tab to switch · Esc to cancel ");
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let [list_area, _, add_area, subscribe_area] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(inner);
            let items: Vec<ListItem> = choices
                .iter()
                .map(|choice| {
                    let title = choice.title.clone().unwrap_or_else(|| "(Untitled)".into());
                    ListItem::new(Line::from(vec![
                        Span::from(title),
                        Span::from(format!("  {}", choice.url)).dim(),
                    ]))
                })
                .collect();
            frame.render_stateful_widget(
                List::new(items).highlight_style(highlight()),
                list_area,
                list,
            );
            if folders.is_none() {
                for (button_area, button) in [
                    (add_area, Button::AddToFolder),
                    (subscribe_area, Button::Subscribe),
                ] {
                    let drawn = draw_button(frame, button_area, button, *focus == button);
                    buttons.push((drawn, button));
                }
            } else {
                draw_button(frame, add_area, Button::AddToFolder, false);
                draw_button(frame, subscribe_area, Button::Subscribe, false);
            }
            if let Some(picker) = folders {
                let width = area.width.saturating_sub(8).min(48);
                let height = picker_height(picker).min(area.height);
                let over = Rect {
                    x: area.x + (area.width - width) / 2,
                    y: area.y + area.height.saturating_sub(height) / 2,
                    width,
                    height,
                };
                let title = "Add to Folder";
                draw_folder_picker(frame, over, picker, title, Button::Subscribe, buttons);
            }
        }
    }
}

fn draw_feeds(frame: &mut Frame, app: &mut App, area: Rect) {
    let unread = |id: &str| app.unread.get(id).copied().unwrap_or(0);
    let items: Vec<ListItem> = app
        .rows
        .iter()
        .map(|listed| match listed {
            Row::All => row("All feeds", app.unread.values().sum()),
            Row::Folder(name) => {
                let count = app
                    .feeds
                    .iter()
                    .filter(|feed| feed.tag_names.contains(name))
                    .map(|feed| unread(&feed.id))
                    .sum();
                let arrow = if app.expanded.contains(name) {
                    "▾"
                } else {
                    "▸"
                };
                row(&format!("{arrow} {name}"), count)
            }
            Row::Feed { id, folder } => {
                let indent = if folder.is_some() { "  " } else { "" };
                let title = app.feed(id).map_or("", |feed| feed.title.as_str());
                row(&format!("{indent}{title}"), unread(id))
            }
        })
        .collect();
    let list = List::new(items)
        .block(pane("Feeds", app.focus == Pane::Feeds))
        .highlight_style(highlight());
    frame.render_stateful_widget(list, area, &mut app.feed_list);
}

fn draw_articles(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = match app.filter {
        ArticleFilter::Unread => "Unread",
        ArticleFilter::All => "All articles",
        ArticleFilter::Starred => "Starred",
    };
    let show_feed = app.selected_feed().is_none();
    let items: Vec<ListItem> = app
        .articles
        .iter()
        .map(|article| {
            let marker = match (article.starred, article.read) {
                (true, _) => Span::from("★ "),
                (false, false) => Span::from("● "),
                (false, true) => Span::from("  "),
            };
            let title = article.title.clone().unwrap_or_else(|| "(Untitled)".into());
            let title = if article.read {
                Span::from(title).dim()
            } else {
                Span::from(title)
            };
            let mut detail = article.published_at.format("%b %-d").to_string();
            if show_feed {
                detail = format!("{} · {detail}", app.feed_title(&article.feed_id));
            }
            ListItem::new(vec![
                Line::from(vec![marker, title]),
                Line::from(format!("  {detail}")).dim(),
            ])
        })
        .collect();
    let empty = items.is_empty();
    let list = List::new(items)
        .block(pane(title, app.focus == Pane::Articles))
        .highlight_style(highlight());
    frame.render_stateful_widget(list, area, &mut app.article_list);
    if empty {
        let message = Paragraph::new("Nothing here. Press f to change the filter, r to refresh.")
            .dim()
            .wrap(Wrap { trim: true });
        frame.render_widget(message, inner(area));
    }
}

fn draw_reader(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut title = vec!["Article"];
    title.extend(app.full_text_note());
    if app.reader.as_ref().is_some_and(|reader| reader.document) {
        title.push("condense");
    }
    let block = pane(&title.join(" · "), app.focus == Pane::Reader);
    let inner = block.inner(area);
    match &mut app.reader {
        Some(reader) => {
            let screen = reader::layout(&reader.parts, inner.width, &app.images, app.true_color);
            reader.max_scroll = (screen.lines.len() as u16).saturating_sub(inner.height);
            reader.scroll = reader.scroll.min(reader.max_scroll);
            reader.links = screen.links;
            let paragraph = Paragraph::new(screen.lines)
                .block(block)
                .scroll((reader.scroll, 0));
            frame.render_widget(paragraph, area);
        }
        None => {
            let hint = Paragraph::new("Select an article and press Enter.")
                .dim()
                .block(block);
            frame.render_widget(hint, area);
        }
    }
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let keys = match app.focus {
        Pane::Feeds => {
            "j/k move · enter open · a add · e rename · M move · d remove · z open/close folder · f filter · r refresh · A all read · q quit"
        }
        Pane::Articles => {
            "j/k move · enter read · m read/unread · s star · A all read · o browser · esc back"
        }
        Pane::Reader => {
            "j/k/space scroll · c full text · / condense · click or L for links · m read · s star · o browser · esc back"
        }
    };
    let status = if app.is_refreshing() {
        "Refreshing…"
    } else {
        app.status.as_str()
    };
    let line = if status.is_empty() {
        Line::from(keys).dim()
    } else {
        Line::from(vec![
            Span::from(status).bold(),
            Span::from(format!("  {keys}")).dim(),
        ])
    };
    frame.render_widget(Paragraph::new(line), area);
}

/// A button: `[ Subscribe ]`, reversed when focused. Returns where it is.
fn draw_button(frame: &mut Frame, area: Rect, button: Button, focused: bool) -> Rect {
    let label = match button {
        Button::AddToFolder => "[ Add to Folder… ]",
        Button::Subscribe => "[ Subscribe ]",
        Button::Move => "[ Move ]",
    };
    let area = Rect {
        width: (label.chars().count() as u16).min(area.width),
        ..area
    };
    let style = if focused {
        highlight()
    } else {
        Style::default()
    };
    frame.render_widget(Paragraph::new(label).style(style), area);
    area
}

/// The folders, a new folder row, a blank line, the button and borders.
fn picker_height(picker: &FolderPicker) -> u16 {
    picker.folders.len() as u16 + 6
}

/// Folders to check, a new folder to name, and the button that confirms:
/// Add to Folder… over the feed selector (Subscribe), or Move to (Move).
fn draw_folder_picker(
    frame: &mut Frame,
    area: Rect,
    picker: &mut FolderPicker,
    title: &str,
    button: Button,
    buttons: &mut Vec<(Rect, Button)>,
) {
    frame.render_widget(Clear, area);
    let block = pane(title, true).title_bottom(" Space to check · Esc to go back ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [list_area, _, subscribe_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    let mut items: Vec<ListItem> = picker
        .folders
        .iter()
        .map(|(name, checked)| {
            let mark = if *checked { "[x] " } else { "[ ] " };
            ListItem::new(format!("{mark}{name}"))
        })
        .collect();
    items.push(match &picker.new_folder {
        Some(name) => ListItem::new(Line::from(vec![
            Span::from("New folder: "),
            Span::from(format!("{name}▏")).bold(),
        ])),
        None => ListItem::new(Line::from("New folder…").italic()),
    });
    frame.render_stateful_widget(
        List::new(items).highlight_style(highlight()),
        list_area,
        &mut picker.list,
    );
    let drawn = draw_button(frame, subscribe_area, button, true);
    buttons.push((drawn, button));
}

/// A dialog with one text box.
fn text_box<'a>(title: &str, label: &'a str, input: &str, hint: &'a str) -> Paragraph<'a> {
    let lines = vec![
        input_line(label, input),
        Line::default(),
        Line::from(hint).dim(),
    ];
    Paragraph::new(lines)
        .block(pane(title, true))
        .wrap(Wrap { trim: true })
}

fn input_line<'a>(label: &'a str, input: &str) -> Line<'a> {
    Line::from(vec![
        Span::from(label),
        Span::from(format!("{input}▏")).bold(),
    ])
}

fn row(title: &str, unread: usize) -> ListItem<'static> {
    let count = if unread > 0 {
        format!(" {unread}")
    } else {
        String::new()
    };
    ListItem::new(Line::from(vec![
        Span::from(title.to_string()),
        Span::from(count).dim(),
    ]))
}

fn pane(title: &str, focused: bool) -> Block<'static> {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "));
    if focused {
        block.border_style(Style::default().add_modifier(Modifier::BOLD))
    } else {
        block.dim()
    }
}

fn highlight() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

/// The area inside a pane's border.
fn inner(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}
