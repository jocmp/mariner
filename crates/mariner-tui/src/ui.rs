//! Drawing: three panes side by side when the terminal is wide enough,
//! otherwise only the focused one.

use mariner_core::ArticleFilter;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, Pane};
use crate::dialog::Dialog;
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
        draw_feeds(frame, app, feeds);
        draw_articles(frame, app, articles);
        draw_reader(frame, app, reader);
    } else {
        match app.focus {
            Pane::Feeds => draw_feeds(frame, app, main),
            Pane::Articles => draw_articles(frame, app, main),
            Pane::Reader => draw_reader(frame, app, main),
        }
    }
    draw_status(frame, app, status);
    if let Some(dialog) = &mut app.dialog {
        draw_dialog(frame, dialog);
    }
}

/// The dialog, centered over everything.
fn draw_dialog(frame: &mut Frame, dialog: &mut Dialog) {
    let screen = frame.area();
    let width = screen.width.saturating_sub(4).min(72);
    let height = match dialog {
        Dialog::Choose { choices, .. } => (choices.len() as u16 + 4).min(screen.height),
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
            let mut lines = vec![
                Line::from(vec![
                    Span::from("URL: "),
                    Span::from(format!("{input}▏")).bold(),
                ]),
                Line::default(),
            ];
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
        Dialog::Adding { url } => {
            let lines = vec![
                Line::from(format!("Finding feeds at {url}…")),
                Line::default(),
                Line::from("Esc to close; the feed is still added").dim(),
            ];
            let paragraph = Paragraph::new(lines)
                .block(pane("Add feed", true))
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
        Dialog::Choose { choices, list } => {
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
            let list_widget = List::new(items)
                .block(
                    pane("Several feeds; pick one", true)
                        .title_bottom(" Enter to add · Esc to cancel "),
                )
                .highlight_style(highlight());
            frame.render_stateful_widget(list_widget, area, list);
        }
    }
}

fn draw_feeds(frame: &mut Frame, app: &mut App, area: Rect) {
    let total: usize = app.unread.values().sum();
    let mut items = vec![row("All feeds", total)];
    items.extend(
        app.feeds
            .iter()
            .map(|feed| row(&feed.title, app.unread.get(&feed.id).copied().unwrap_or(0))),
    );
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
    let block = pane("Article", app.focus == Pane::Reader);
    match &app.reader {
        Some(reader) => {
            let width = area.width.saturating_sub(2);
            let lines = reader::render(&reader.parts, width, &app.images, app.true_color);
            let paragraph = Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false })
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
        Pane::Feeds => "j/k move · enter open · a add · f filter · r refresh · A all read · q quit",
        Pane::Articles => {
            "j/k move · enter read · m read/unread · s star · A all read · o browser · esc back"
        }
        Pane::Reader => "j/k/space scroll · m read/unread · s star · o browser · esc back",
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
