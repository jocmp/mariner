//! The interactive reader: state, keys, and the event loop.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use mariner_core::{Article, ArticleContent, ArticleFilter, ArticleScope, Feed, FeedChoice};
use mariner_feeds::fetch::HttpFetcher;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use crate::dialog::{Action, Button, Dialog, Target};
use crate::images::Images;
use crate::reader::{self, LinkRange, Part};
use crate::{LocalAccount, halfblock, ui};

/// Articles loaded into the list at once.
const PAGE: usize = 500;

/// Runs the reader until the user quits.
pub fn run(
    runtime: &Runtime,
    account: Arc<LocalAccount>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(runtime, account, HttpFetcher::new()?);
    app.reload();
    let mut terminal = ratatui::init();
    let result = (|| -> std::io::Result<()> {
        execute!(std::io::stdout(), EnableMouseCapture)?;
        while !app.quit {
            terminal.draw(|frame| ui::draw(frame, &mut app))?;
            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key),
                    Event::Mouse(mouse) => app.on_mouse(mouse),
                    _ => {}
                }
            }
            app.tick();
        }
        Ok(())
    })();
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    Ok(result?)
}

/// Opens a URL in the browser.
type Opener = Box<dyn Fn(&str) -> std::io::Result<()>>;

/// The pane with the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Feeds,
    Articles,
    Reader,
}

pub struct App<'a> {
    runtime: &'a Runtime,
    account: Arc<LocalAccount>,
    pub feeds: Vec<Feed>,
    pub unread: HashMap<String, usize>,
    /// The feeds pane's rows: every feed, then each folder with its feeds,
    /// then feeds in no folder.
    pub rows: Vec<Row>,
    /// Folders that show their feeds.
    pub expanded: HashSet<String>,
    pub feed_list: ListState,
    pub filter: ArticleFilter,
    pub articles: Vec<Article>,
    pub article_list: ListState,
    pub reader: Option<Reader>,
    pub images: Images,
    /// Whether to draw images in 24-bit color, or the 256 standard colors.
    pub true_color: bool,
    pub focus: Pane,
    /// Where each pane was last drawn, for mouse clicks; `None` when hidden.
    pub areas: Areas,
    /// Where the dialog's buttons were last drawn, for mouse clicks.
    pub buttons: Vec<(Rect, Button)>,
    pub status: String,
    /// A dialog over the panes, which takes every key while it's open.
    pub dialog: Option<Dialog>,
    /// Fetches article pages for their full text.
    fetcher: HttpFetcher,
    /// What extracting full texts led to this session, by article id.
    full_texts: HashMap<String, FullText>,
    /// Extractions under way, by article id.
    extracting: HashMap<String, JoinHandle<mariner_core::Result<Option<String>>>>,
    /// Opens a URL in the browser; swappable so tests don't.
    opener: Opener,
    refreshing: Option<JoinHandle<mariner_core::Result<()>>>,
    finding: Option<JoinHandle<mariner_core::Result<Vec<FeedChoice>>>>,
    subscribing: Option<JoinHandle<mariner_core::Result<Feed>>>,
    quit: bool,
}

/// Where the panes were last drawn, borders included.
#[derive(Debug, Default, Clone, Copy)]
pub struct Areas {
    pub feeds: Option<Rect>,
    pub articles: Option<Rect>,
    pub reader: Option<Rect>,
}

/// A row of the feeds pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    All,
    Folder(String),
    /// A feed by id, under `folder` or in none.
    Feed {
        id: String,
        folder: Option<String>,
    },
}

/// What extracting an article's full text led to.
enum FullText {
    Found(String),
    /// The page had no article.
    Missing,
    Failed(String),
}

/// The open article.
pub struct Reader {
    pub article: Article,
    /// The content the feed gave, to switch back to from the full text.
    pub feed_html: Option<String>,
    /// Whether the full text is showing.
    pub full_text: bool,
    /// Whether the full text was asked for (with c, or by the feed's
    /// setting), so it shows once it's extracted.
    pub wants_full_text: bool,
    /// Whether condense's output shows instead of the rendered article.
    pub document: bool,
    pub parts: Vec<Part>,
    pub scroll: u16,
    /// The furthest the article scrolls at the last drawn size.
    pub max_scroll: u16,
    /// The links on each line as last laid out.
    pub links: Vec<Vec<LinkRange>>,
}

impl<'a> App<'a> {
    fn new(runtime: &'a Runtime, account: Arc<LocalAccount>, fetcher: HttpFetcher) -> Self {
        let images = Images::new(fetcher.clone());
        App {
            runtime,
            account,
            feeds: Vec::new(),
            unread: HashMap::new(),
            rows: vec![Row::All],
            expanded: HashSet::new(),
            feed_list: ListState::default().with_selected(Some(0)),
            filter: ArticleFilter::Unread,
            articles: Vec::new(),
            article_list: ListState::default(),
            reader: None,
            images,
            true_color: halfblock::true_color_terminal(),
            focus: Pane::Feeds,
            areas: Areas::default(),
            buttons: Vec::new(),
            status: String::new(),
            dialog: None,
            fetcher,
            full_texts: HashMap::new(),
            extracting: HashMap::new(),
            opener: Box::new(open_in_system_browser),
            refreshing: None,
            finding: None,
            subscribing: None,
            quit: false,
        }
    }

    pub fn is_refreshing(&self) -> bool {
        self.refreshing.is_some()
    }

    /// The selected feed, or `None` for every feed or a folder.
    pub fn selected_feed(&self) -> Option<&Feed> {
        match self.selected_row() {
            Row::Feed { id, .. } => self.feed(id),
            Row::All | Row::Folder(_) => None,
        }
    }

    pub fn feed(&self, id: &str) -> Option<&Feed> {
        self.feeds.iter().find(|feed| feed.id == id)
    }

    /// What the reader's title says about the full text: showing, being
    /// extracted, or why it isn't showing.
    pub fn full_text_note(&self) -> Option<&'static str> {
        let reader = self.reader.as_ref()?;
        if reader.full_text {
            return Some("full text");
        }
        if !reader.wants_full_text {
            return None;
        }
        let id = &reader.article.id;
        if self.extracting.contains_key(id) {
            return Some("getting full text…");
        }
        match self.full_texts.get(id) {
            Some(FullText::Missing) => Some("no full text found"),
            Some(FullText::Failed(_)) => Some("full text failed"),
            Some(FullText::Found(_)) | None => None,
        }
    }

    fn selected_row(&self) -> &Row {
        let row = self.feed_list.selected().unwrap_or(0);
        self.rows.get(row).unwrap_or(&Row::All)
    }

    /// Where the article list comes from.
    fn scope(&self) -> ArticleScope {
        match self.selected_row() {
            Row::All => ArticleScope::All,
            Row::Folder(name) => ArticleScope::Tag(name.clone()),
            Row::Feed { id, .. } => ArticleScope::Feed(id.clone()),
        }
    }

    pub fn feed_title(&self, feed_id: &str) -> &str {
        self.feeds
            .iter()
            .find(|feed| feed.id == feed_id)
            .map_or("", |feed| feed.title.as_str())
    }

    fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if let Some(dialog) = &mut self.dialog {
            let action = dialog.on_key(key);
            self.act(action);
            return;
        }
        if self.focus == Pane::Feeds {
            match key.code {
                KeyCode::Char('e') => return self.ask_rename(),
                KeyCode::Char('d') => return self.ask_remove(),
                KeyCode::Char('M') => return self.ask_move(),
                KeyCode::Char('z') => return self.toggle_folder(),
                _ => {}
            }
        }
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('a') => self.dialog = Some(Dialog::add()),
            KeyCode::Char('j') | KeyCode::Down => self.move_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_by(-1),
            KeyCode::Char(' ') | KeyCode::PageDown => self.move_by(10),
            KeyCode::Char('b') | KeyCode::PageUp => self.move_by(-10),
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => self.forward(),
            KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => self.back(),
            KeyCode::Tab => self.cycle_focus(),
            KeyCode::Char('f') => self.cycle_filter(),
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('m') => self.toggle_read(),
            KeyCode::Char('s') => self.toggle_star(),
            KeyCode::Char('A') => self.mark_all_read(),
            KeyCode::Char('o') => self.open_in_browser(),
            KeyCode::Char('L') => self.show_links(),
            KeyCode::Char('c') => self.toggle_full_text(),
            KeyCode::Char('/') => self.toggle_document(),
            _ => {}
        }
    }

    /// Does what a dialog asked.
    fn act(&mut self, action: Action) {
        match action {
            Action::None => {}
            Action::Close => self.dialog = None,
            Action::Find(url) => self.find_feeds(url),
            Action::Subscribe {
                url,
                title,
                folders,
            } => self.subscribe(url, title, folders),
            Action::Open(url) => {
                self.dialog = None;
                self.open_url(&url);
            }
            Action::Rename(target, name) => self.rename(target, name),
            Action::Remove(target) => self.remove(target),
            Action::Move { feed_id, folders } => self.move_feed(feed_id, folders),
        }
    }

    /// Notices a finished refresh, add or extraction.
    fn tick(&mut self) {
        let finished: Vec<String> = self
            .extracting
            .iter()
            .filter(|(_, handle)| handle.is_finished())
            .map(|(id, _)| id.clone())
            .collect();
        for id in finished {
            let handle = self.extracting.remove(&id).expect("listed");
            let result = match self.runtime.block_on(handle) {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            self.extracted(id, result);
        }
        if self.finding.as_ref().is_some_and(JoinHandle::is_finished) {
            let handle = self.finding.take().expect("checked");
            let result = match self.runtime.block_on(handle) {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            self.found(result);
        }
        if self
            .subscribing
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
        {
            let handle = self.subscribing.take().expect("checked");
            let result = match self.runtime.block_on(handle) {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            self.subscribed(result);
        }
        if self
            .refreshing
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
        {
            let handle = self.refreshing.take().expect("checked");
            self.status = match self.runtime.block_on(handle) {
                Ok(Ok(())) => "Refreshed".into(),
                Ok(Err(error)) => format!("Refresh failed: {error}"),
                Err(error) => format!("Refresh failed: {error}"),
            };
            self.reload();
        }
    }

    /// Finds the feeds at `url` in the background.
    fn find_feeds(&mut self, url: String) {
        let account = Arc::clone(&self.account);
        let task_url = url.clone();
        self.finding = Some(
            self.runtime
                .spawn(async move { account.find_feeds(&task_url).await }),
        );
        self.dialog = Some(Dialog::Finding { url });
    }

    /// Shows the feeds found to pick from, even just one, or the URL again
    /// with what went wrong.
    fn found(&mut self, result: Result<Vec<FeedChoice>, String>) {
        let Some(Dialog::Finding { url }) = &self.dialog else {
            // Closed while finding.
            return;
        };
        let retry = |message: &str| Dialog::Add {
            input: url.clone(),
            message: Some(message.to_string()),
        };
        self.dialog = Some(match result {
            Ok(choices) if choices.is_empty() => retry("No feed found there."),
            Ok(choices) => Dialog::choose(choices, self.folder_names()),
            Err(error) => retry(&error),
        });
    }

    /// Every folder, by name.
    fn folder_names(&self) -> Vec<String> {
        match self.runtime.block_on(self.account.tags()) {
            Ok(tags) => tags.into_iter().map(|tag| tag.name).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Subscribes to the feed at `url` in the background, then, as a
    /// separate change, adds it to `folders`.
    fn subscribe(&mut self, url: String, title: Option<String>, folders: Vec<String>) {
        let account = Arc::clone(&self.account);
        self.subscribing = Some(self.runtime.spawn(async move {
            let feed = account.subscribe(&url, None).await?;
            if folders.is_empty() {
                return Ok(feed);
            }
            let mut tags = feed.tag_names.clone();
            tags.extend(folders);
            let feed = account.update_feed(&feed, &feed.title, &tags).await?;
            for folder in &feed.tag_names {
                account.set_tag_expanded(folder, true).await?;
            }
            Ok(feed)
        }));
        let title = title.unwrap_or_else(|| "the feed".into());
        self.dialog = Some(Dialog::Subscribing { title });
    }

    /// Selects the new feed, under its first folder.
    fn subscribed(&mut self, result: Result<Feed, String>) {
        if matches!(self.dialog, Some(Dialog::Subscribing { .. })) {
            self.dialog = None;
        }
        match result {
            Ok(feed) => {
                self.status = format!("Subscribed to {}", feed.title);
                self.reload();
                self.select_feed(&feed.id, feed.tag_names.first().map(String::as_str));
                self.focus = Pane::Feeds;
            }
            Err(error) => self.status = format!("Couldn't subscribe: {error}"),
        }
    }

    /// Reloads feeds, counts and the article list, keeping selections.
    fn reload(&mut self) {
        let selected = self.selected_row().clone();
        match self.runtime.block_on(self.account.feeds()) {
            Ok(feeds) => self.feeds = feeds,
            Err(error) => self.status = error.to_string(),
        }
        if let Ok(unread) = self.runtime.block_on(self.account.unread_counts()) {
            self.unread = unread;
        }
        match self.runtime.block_on(self.account.tags()) {
            Ok(tags) => {
                self.expanded = tags
                    .into_iter()
                    .filter(|tag| tag.expanded)
                    .map(|tag| tag.name)
                    .collect();
            }
            Err(error) => self.status = error.to_string(),
        }
        self.rows = rows(&self.feeds, &self.expanded);
        // A feed in a folder that closed leaves its folder selected.
        let folder = match &selected {
            Row::Feed {
                folder: Some(folder),
                ..
            } => Some(Row::Folder(folder.clone())),
            _ => None,
        };
        let row = self
            .rows
            .iter()
            .position(|row| *row == selected)
            .or_else(|| {
                self.rows
                    .iter()
                    .position(|row| Some(row) == folder.as_ref())
            })
            .unwrap_or(0);
        self.feed_list.select(Some(row));
        self.load_articles();
    }

    /// Selects a feed's row, under `folder` when it's listed there, and
    /// lists its articles.
    fn select_feed(&mut self, id: &str, folder: Option<&str>) {
        let is_feed = |row: &Row| matches!(row, Row::Feed { id: row_id, .. } if row_id == id);
        let row = self
            .rows
            .iter()
            .position(|row| {
                is_feed(row) && matches!(row, Row::Feed { folder: f, .. } if f.as_deref() == folder)
            })
            .or_else(|| self.rows.iter().position(is_feed));
        if let Some(row) = row {
            self.feed_list.select(Some(row));
            self.article_list.select(None);
            self.load_articles();
        }
    }

    fn load_articles(&mut self) {
        let scope = self.scope();
        let selected_id = self
            .article_list
            .selected()
            .and_then(|row| self.articles.get(row))
            .map(|article| article.id.clone());
        let articles = self
            .runtime
            .block_on(self.account.articles(self.filter, &scope, PAGE, 0));
        match articles {
            Ok(articles) => self.articles = articles,
            Err(error) => self.status = error.to_string(),
        }
        let row = selected_id
            .and_then(|id| self.articles.iter().position(|article| article.id == id))
            .or((!self.articles.is_empty()).then_some(0));
        self.article_list.select(row);
    }

    fn move_by(&mut self, delta: i32) {
        match self.focus {
            Pane::Feeds => {
                let row = step(
                    self.feed_list.selected().unwrap_or(0),
                    delta,
                    self.rows.len(),
                );
                if Some(row) != self.feed_list.selected() {
                    self.feed_list.select(Some(row));
                    self.article_list.select(None);
                    self.load_articles();
                }
            }
            Pane::Articles => {
                if !self.articles.is_empty() {
                    let row = step(
                        self.article_list.selected().unwrap_or(0),
                        delta,
                        self.articles.len(),
                    );
                    self.article_list.select(Some(row));
                }
            }
            Pane::Reader => {
                if let Some(reader) = &mut self.reader {
                    reader.scroll = reader
                        .scroll
                        .saturating_add_signed(delta as i16)
                        .min(reader.max_scroll);
                }
            }
        }
    }

    fn forward(&mut self) {
        match self.focus {
            Pane::Feeds => self.focus = Pane::Articles,
            Pane::Articles => self.open_article(),
            Pane::Reader => {}
        }
    }

    fn back(&mut self) {
        self.focus = match self.focus {
            Pane::Feeds | Pane::Articles => Pane::Feeds,
            Pane::Reader => Pane::Articles,
        };
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Pane::Feeds => Pane::Articles,
            Pane::Articles if self.reader.is_some() => Pane::Reader,
            Pane::Articles | Pane::Reader => Pane::Feeds,
        };
    }

    fn cycle_filter(&mut self) {
        self.filter = match self.filter {
            ArticleFilter::Unread => ArticleFilter::All,
            ArticleFilter::All => ArticleFilter::Starred,
            ArticleFilter::Starred => ArticleFilter::Unread,
        };
        self.article_list.select(None);
        self.load_articles();
    }

    /// Shows the selected article and marks it read.
    fn open_article(&mut self) {
        let Some(article) = self.selected_article().cloned() else {
            return;
        };
        match self.runtime.block_on(self.account.article(&article.id)) {
            Ok(Some(content)) => {
                let parts = reader::parts(&content, self.feed_title(&article.feed_id));
                self.images
                    .request(self.runtime, reader::image_urls(&parts));
                let sticky = self
                    .feed(&article.feed_id)
                    .is_some_and(|feed| feed.sticky_full_content);
                self.reader = Some(Reader {
                    article: content.article,
                    feed_html: content.content_html,
                    full_text: false,
                    wants_full_text: false,
                    document: false,
                    parts,
                    scroll: 0,
                    max_scroll: 0,
                    links: Vec::new(),
                });
                self.focus = Pane::Reader;
                if !article.read {
                    self.set_read(&article.id, true);
                }
                if sticky {
                    self.request_full_text();
                }
            }
            Ok(None) => self.status = "That article is gone".into(),
            Err(error) => self.status = error.to_string(),
        }
    }

    /// The article the reader shows, or else the one selected in the list.
    fn current_article(&self) -> Option<&Article> {
        match (&self.reader, self.focus) {
            (Some(reader), Pane::Reader) => Some(&reader.article),
            _ => self.selected_article(),
        }
    }

    fn selected_article(&self) -> Option<&Article> {
        self.article_list
            .selected()
            .and_then(|row| self.articles.get(row))
    }

    fn toggle_read(&mut self) {
        if let Some(article) = self.current_article() {
            let (id, read) = (article.id.clone(), !self.is_read(&article.id, article.read));
            self.set_read(&id, read);
        }
    }

    fn toggle_star(&mut self) {
        let Some(article) = self.current_article() else {
            return;
        };
        let id = vec![article.id.clone()];
        let starred = !self
            .articles
            .iter()
            .find(|listed| listed.id == article.id)
            .map_or(article.starred, |listed| listed.starred);
        let result = if starred {
            self.runtime.block_on(self.account.add_star(&id))
        } else {
            self.runtime.block_on(self.account.remove_star(&id))
        };
        if let Err(error) = result {
            self.status = error.to_string();
        }
        self.update_listed(&id[0], |article| article.starred = starred);
    }

    fn set_read(&mut self, id: &str, read: bool) {
        let ids = vec![id.to_string()];
        let result = if read {
            self.runtime.block_on(self.account.mark_read(&ids))
        } else {
            self.runtime.block_on(self.account.mark_unread(&ids))
        };
        if let Err(error) = result {
            self.status = error.to_string();
        }
        self.update_listed(id, |article| article.read = read);
        if let Ok(unread) = self.runtime.block_on(self.account.unread_counts()) {
            self.unread = unread;
        }
    }

    /// Updates the listed copy in place, so the list doesn't jump while
    /// reading through unread articles.
    fn update_listed(&mut self, id: &str, change: impl Fn(&mut Article)) {
        if let Some(article) = self.articles.iter_mut().find(|article| article.id == id) {
            change(article);
        }
        if let Some(reader) = self
            .reader
            .as_mut()
            .filter(|reader| reader.article.id == id)
        {
            change(&mut reader.article);
        }
    }

    fn is_read(&self, id: &str, fallback: bool) -> bool {
        self.articles
            .iter()
            .find(|article| article.id == id)
            .map_or(fallback, |article| article.read)
    }

    fn mark_all_read(&mut self) {
        let scope = self.scope();
        match self.runtime.block_on(self.account.mark_all_read(&scope)) {
            Ok(()) => self.status = "Marked all read".into(),
            Err(error) => self.status = error.to_string(),
        }
        self.reload();
    }

    fn refresh(&mut self) {
        if self.refreshing.is_some() {
            return;
        }
        let account = Arc::clone(&self.account);
        self.refreshing = Some(
            self.runtime
                .spawn(async move { account.refresh(None).await }),
        );
        self.status = "Refreshing…".into();
    }

    fn open_in_browser(&mut self) {
        if let Some(url) = self
            .current_article()
            .and_then(|article| article.url.clone())
        {
            self.open_url(&url);
        }
    }

    fn open_url(&mut self, url: &str) {
        match (self.opener)(url) {
            Ok(()) => self.status = format!("Opened {url}"),
            Err(error) => self.status = format!("Couldn't open {url}: {error}"),
        }
    }

    /// Switches the open article between the feed's content and the full
    /// text from its page, and remembers the choice for its feed. Asking
    /// again after a failure tries again, and asking while it's being
    /// extracted stops waiting for it.
    fn toggle_full_text(&mut self) {
        let Some(reader) = &mut self.reader else {
            return;
        };
        let feed_id = reader.article.feed_id.clone();
        let waiting = reader.wants_full_text && self.extracting.contains_key(&reader.article.id);
        if reader.full_text || waiting {
            reader.wants_full_text = false;
            self.show_content(false);
            self.set_sticky_full_text(&feed_id, false);
            return;
        }
        self.request_full_text();
        self.set_sticky_full_text(&feed_id, true);
    }

    /// Stores whether a feed's articles open in full text, when it changed.
    fn set_sticky_full_text(&mut self, feed_id: &str, enabled: bool) {
        if self
            .feed(feed_id)
            .is_none_or(|feed| feed.sticky_full_content == enabled)
        {
            return;
        }
        let result = self
            .runtime
            .block_on(self.account.set_sticky_full_content(feed_id, enabled));
        match result {
            Ok(()) => {
                if let Some(feed) = self.feeds.iter_mut().find(|feed| feed.id == feed_id) {
                    feed.sticky_full_content = enabled;
                }
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    /// Shows the open article's full text, extracting it in the background
    /// unless it already was.
    fn request_full_text(&mut self) {
        let Some(reader) = &mut self.reader else {
            return;
        };
        reader.wants_full_text = true;
        let id = reader.article.id.clone();
        if let Some(FullText::Found(_)) = self.full_texts.get(&id) {
            return self.show_content(true);
        }
        let Some(url) = reader.article.url.clone() else {
            self.full_texts
                .insert(id, FullText::Failed("no link to read from".into()));
            self.status = "This article has no link to read from".into();
            return;
        };
        if self.extracting.contains_key(&id) {
            return;
        }
        let fetcher = self.fetcher.clone();
        let task = self
            .runtime
            .spawn(async move { mariner_core::extract::full_content(&fetcher, &url).await });
        self.extracting.insert(id, task);
        self.status = "Getting the full text…".into();
    }

    /// Keeps an extraction's result, showing it if its article is open and
    /// waiting for it.
    fn extracted(&mut self, id: String, result: Result<Option<String>, String>) {
        let waiting = self
            .reader
            .as_ref()
            .is_some_and(|reader| reader.article.id == id && reader.wants_full_text);
        let full_text = match result {
            Ok(Some(html)) => FullText::Found(html),
            Ok(None) => FullText::Missing,
            Err(error) => FullText::Failed(error),
        };
        if waiting {
            self.status = match &full_text {
                FullText::Found(_) => "Showing the full text; c for the feed's".into(),
                FullText::Missing => "No full text found for this article; c to try again".into(),
                FullText::Failed(error) => {
                    format!("Couldn't get the full text: {error}; c to try again")
                }
            };
        }
        let found = matches!(full_text, FullText::Found(_));
        self.full_texts.insert(id, full_text);
        if waiting && found {
            self.show_content(true);
        }
    }

    /// Shows the open article's full text, or the feed's content.
    fn show_content(&mut self, full_text: bool) {
        if let Some(reader) = &mut self.reader {
            reader.full_text = full_text;
        }
        self.rebuild_reader();
    }

    /// Switches the open article between its rendering and condense's
    /// output for the same content.
    fn toggle_document(&mut self) {
        if let Some(reader) = &mut self.reader {
            reader.document = !reader.document;
        }
        self.rebuild_reader();
    }

    /// Rebuilds the reader for its content (the feed's or the full text) and
    /// its view (rendered, or condense's output).
    fn rebuild_reader(&mut self) {
        let Some(reader) = &self.reader else { return };
        let html = if reader.full_text {
            match self.full_texts.get(&reader.article.id) {
                Some(FullText::Found(html)) => Some(html.clone()),
                _ => None,
            }
        } else {
            reader.feed_html.clone()
        };
        let content = ArticleContent {
            article: reader.article.clone(),
            content_html: html,
        };
        let parts = if reader.document {
            reader::document_parts(&content)
        } else {
            let parts = reader::parts(&content, self.feed_title(&reader.article.feed_id));
            self.images
                .request(self.runtime, reader::image_urls(&parts));
            parts
        };
        if let Some(reader) = &mut self.reader {
            reader.parts = parts;
            reader.scroll = 0;
        }
    }

    /// Asks for a new name for the selected feed or folder.
    fn ask_rename(&mut self) {
        let (target, name) = match self.selected_row() {
            Row::All => return,
            Row::Folder(name) => (Target::Folder(name.clone()), name.clone()),
            Row::Feed { id, .. } => match self.feed(id) {
                Some(feed) => (Target::Feed(id.clone()), feed.title.clone()),
                None => return,
            },
        };
        self.dialog = Some(Dialog::Rename {
            target,
            input: name,
        });
    }

    /// Asks before removing the selected feed or folder.
    fn ask_remove(&mut self) {
        let (target, name) = match self.selected_row() {
            Row::All => return,
            Row::Folder(name) => (Target::Folder(name.clone()), name.clone()),
            Row::Feed { id, .. } => match self.feed(id) {
                Some(feed) => (Target::Feed(id.clone()), feed.title.clone()),
                None => return,
            },
        };
        self.dialog = Some(Dialog::Remove { target, name });
    }

    /// Offers every folder to move the selected feed to, its own checked.
    fn ask_move(&mut self) {
        let Some(feed) = self.selected_feed().cloned() else {
            self.status = "Select a feed to move".into();
            return;
        };
        let folders = self.folder_names();
        self.dialog = Some(Dialog::move_to(feed.id, folders, &feed.tag_names));
    }

    fn rename(&mut self, target: Target, name: String) {
        self.dialog = None;
        let result = match &target {
            Target::Feed(id) => {
                let Some(feed) = self.feed(id).cloned() else {
                    return;
                };
                self.runtime
                    .block_on(self.account.update_feed(&feed, &name, &feed.tag_names))
                    .map(|_| ())
            }
            Target::Folder(old) => self.runtime.block_on(self.account.update_tag(old, &name)),
        };
        match result {
            Ok(()) => self.status = format!("Renamed to {name}"),
            Err(error) => self.status = format!("Couldn't rename: {error}"),
        }
        self.reload();
        if matches!(target, Target::Folder(_))
            && let Some(row) = self
                .rows
                .iter()
                .position(|row| *row == Row::Folder(name.clone()))
        {
            self.feed_list.select(Some(row));
            self.load_articles();
        }
    }

    fn remove(&mut self, target: Target) {
        self.dialog = None;
        let result = match &target {
            Target::Feed(id) => {
                let Some(feed) = self.feed(id).cloned() else {
                    return;
                };
                let result = self.runtime.block_on(self.account.remove_feed(&feed));
                if result.is_ok() {
                    self.status = format!("Removed {}", feed.title);
                    if self
                        .reader
                        .as_ref()
                        .is_some_and(|reader| reader.article.feed_id == feed.id)
                    {
                        self.reader = None;
                    }
                }
                result
            }
            Target::Folder(name) => {
                let result = self.runtime.block_on(self.account.remove_tag(name));
                if result.is_ok() {
                    self.status = format!("Removed the folder {name}; its feeds stay");
                }
                result
            }
        };
        if let Err(error) = result {
            self.status = format!("Couldn't remove: {error}");
        }
        self.reload();
    }

    /// Opens or closes the selected folder.
    fn toggle_folder(&mut self) {
        if let Row::Folder(name) = self.selected_row().clone() {
            let expanded = !self.expanded.contains(&name);
            self.set_folder_expanded(&name, expanded);
            self.reload();
        }
    }

    fn set_folder_expanded(&mut self, name: &str, expanded: bool) {
        let result = self
            .runtime
            .block_on(self.account.set_tag_expanded(name, expanded));
        if let Err(error) = result {
            self.status = error.to_string();
        }
    }

    /// Puts a feed in exactly `folders`, opening them so the feed stays in
    /// sight. Folders it leaves empty go.
    fn move_feed(&mut self, feed_id: String, folders: Vec<String>) {
        self.dialog = None;
        let Some(feed) = self.feed(&feed_id).cloned() else {
            return;
        };
        let result = self
            .runtime
            .block_on(self.account.update_feed(&feed, &feed.title, &folders));
        match result {
            Ok(moved) => {
                for folder in &moved.tag_names {
                    self.set_folder_expanded(folder, true);
                }
                self.status = if moved.tag_names.is_empty() {
                    format!("Moved {} out of its folders", feed.title)
                } else {
                    format!("Moved {} to {}", feed.title, moved.tag_names.join(", "))
                };
                self.reload();
                self.select_feed(&feed_id, moved.tag_names.first().map(String::as_str));
            }
            Err(error) => self.status = format!("Couldn't move: {error}"),
        }
    }

    /// Lists the open article's links to pick one.
    fn show_links(&mut self) {
        let Some(reader) = &self.reader else { return };
        let links = reader::links(&reader.parts);
        if links.is_empty() {
            self.status = "This article has no links".into();
        } else {
            self.dialog = Some(Dialog::links(links));
        }
    }

    fn on_mouse(&mut self, mouse: MouseEvent) {
        if let Some(dialog) = &mut self.dialog {
            let clicked = self.buttons.iter().find(|(area, _)| {
                area.contains(ratatui::layout::Position::new(mouse.column, mouse.row))
            });
            if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                && let Some(&(_, button)) = clicked
            {
                let action = dialog.press(button);
                self.act(action);
            }
            return;
        }
        let (column, row) = (mouse.column, mouse.row);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => self.click(column, row),
            MouseEventKind::ScrollDown => self.scroll_at(column, row, 3),
            MouseEventKind::ScrollUp => self.scroll_at(column, row, -3),
            _ => {}
        }
    }

    /// Selects the feed or article clicked, opening an article, or opens
    /// the link clicked in the reader.
    fn click(&mut self, column: u16, row: u16) {
        let Some((pane, x, y)) = self.pane_at(column, row) else {
            return;
        };
        self.focus = pane;
        match pane {
            Pane::Feeds => {
                let index = self.feed_list.offset() + usize::from(y);
                let on_arrow = x < 2 && matches!(self.rows.get(index), Some(Row::Folder(_)));
                if on_arrow {
                    self.feed_list.select(Some(index));
                    self.toggle_folder();
                } else if index < self.rows.len() && Some(index) != self.feed_list.selected() {
                    self.feed_list.select(Some(index));
                    self.article_list.select(None);
                    self.load_articles();
                }
            }
            Pane::Articles => {
                // Each article takes two lines.
                let index = self.article_list.offset() + usize::from(y / 2);
                if index < self.articles.len() {
                    self.article_list.select(Some(index));
                    self.open_article();
                }
            }
            Pane::Reader => {
                let Some(reader) = &self.reader else { return };
                let line = usize::from(reader.scroll) + usize::from(y);
                let url = reader.links.get(line).and_then(|links| {
                    links
                        .iter()
                        .find(|link| link.start <= x && x < link.end)
                        .map(|link| link.url.to_string())
                });
                if let Some(url) = url {
                    self.open_url(&url);
                }
            }
        }
    }

    /// Scrolls the pane under the pointer.
    fn scroll_at(&mut self, column: u16, row: u16, delta: i32) {
        if let Some((pane, _, _)) = self.pane_at(column, row) {
            let focus = self.focus;
            self.focus = pane;
            self.move_by(delta);
            if pane != Pane::Reader {
                self.focus = focus;
            }
        }
    }

    /// The pane at a screen position, and the position inside its border.
    fn pane_at(&self, column: u16, row: u16) -> Option<(Pane, u16, u16)> {
        [
            (Pane::Feeds, self.areas.feeds),
            (Pane::Articles, self.areas.articles),
            (Pane::Reader, self.areas.reader),
        ]
        .into_iter()
        .find_map(|(pane, area)| {
            let area = area?;
            let inside = column > area.x
                && column < area.x + area.width.saturating_sub(1)
                && row > area.y
                && row < area.y + area.height.saturating_sub(1);
            inside.then(|| (pane, column - area.x - 1, row - area.y - 1))
        })
    }
}

/// The feeds pane's rows: every feed, each folder followed by its feeds
/// when it's open, then the feeds in no folder. A feed with several tags is under each.
fn rows(feeds: &[Feed], expanded: &HashSet<String>) -> Vec<Row> {
    let mut folders: Vec<&String> = feeds.iter().flat_map(|feed| &feed.tag_names).collect();
    folders.sort_by_key(|name| name.to_lowercase());
    folders.dedup();
    let mut rows = vec![Row::All];
    for folder in folders {
        rows.push(Row::Folder(folder.clone()));
        if !expanded.contains(folder) {
            continue;
        }
        rows.extend(
            feeds
                .iter()
                .filter(|feed| feed.tag_names.contains(folder))
                .map(|feed| Row::Feed {
                    id: feed.id.clone(),
                    folder: Some(folder.clone()),
                }),
        );
    }
    rows.extend(
        feeds
            .iter()
            .filter(|feed| feed.tag_names.is_empty())
            .map(|feed| Row::Feed {
                id: feed.id.clone(),
                folder: None,
            }),
    );
    rows
}

/// Opens `url` with the system's opener (`open` on macOS, `xdg-open`
/// elsewhere).
fn open_in_system_browser(url: &str) -> std::io::Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(url)
        .spawn()
        .map(|_| ())
}

/// `row + delta`, kept within `0..rows`.
fn step(row: usize, delta: i32, rows: usize) -> usize {
    let last = rows.saturating_sub(1) as i64;
    (row as i64 + i64::from(delta)).clamp(0, last) as usize
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent};

    use super::*;
    use mariner_core::{Account, Database, Kind};
    use mariner_feeds::fetch::HttpFetcher;

    /// An account with one feed of two articles, seeded without a network.
    fn account(runtime: &Runtime, dir: &std::path::Path) -> Arc<LocalAccount> {
        runtime.block_on(async {
            let database = Database::open(dir.join("local.db"), Kind::Account).await.unwrap();
            database
                .write(|connection| {
                    connection.execute_batch(
                        "INSERT INTO feeds (id, subscription_id, title, feed_url)
                           VALUES ('f', 'f', 'Example Feed', 'https://example.com/feed.xml');
                         INSERT INTO articles (id, feed_id, title, url, content_html, published_at, created_at)
                           VALUES ('a', 'f', 'First Post', 'https://example.com/1',
                                   '<p>Hello <b>world</b>, see <a href=\"https://example.com/more\">more</a>.</p><ul><li>One</li></ul>', 2000000000, 2000000000),
                                  ('b', 'f', 'Second Post', 'https://example.com/2', '<p>Bye.</p>', 1900000000, 1900000000);
                         INSERT INTO article_statuses (article_id, updated_at) VALUES ('a', 1), ('b', 1);",
                    )
                })
                .await
                .unwrap();
            Arc::new(Account::local(database, HttpFetcher::new().unwrap()))
        })
    }

    fn render(app: &mut App, width: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, 20)).unwrap();
        terminal.draw(|frame| ui::draw(frame, app)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::from(code));
    }

    #[test]
    fn shows_three_panes_when_wide_and_one_when_narrow() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = App::new(
            &runtime,
            account(&runtime, dir.path()),
            HttpFetcher::new().unwrap(),
        );
        app.reload();

        let wide = render(&mut app, 140);
        assert!(
            wide.contains(" Feeds ") && wide.contains(" Unread ") && wide.contains(" Article ")
        );
        assert!(wide.contains("Example Feed 2"), "{wide}");

        let narrow = render(&mut app, 80);
        assert!(
            narrow.contains(" Feeds ") && !narrow.contains(" Unread "),
            "{narrow}"
        );
    }

    #[test]
    fn walks_from_feeds_to_the_reader_and_back() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = App::new(
            &runtime,
            account(&runtime, dir.path()),
            HttpFetcher::new().unwrap(),
        );
        app.reload();

        press(&mut app, KeyCode::Enter);
        let articles = render(&mut app, 80);
        assert!(
            articles.contains("First Post") && articles.contains("Second Post"),
            "{articles}"
        );

        press(&mut app, KeyCode::Enter);
        let reader = render(&mut app, 80);
        assert!(
            reader.contains("First Post") && reader.contains("Hello world, see more."),
            "{reader}"
        );
        assert!(reader.contains("• One"), "{reader}");
        assert_eq!(app.unread.get("f"), Some(&1), "opening marks it read");

        if std::env::var_os("MARINER_PRINT_UI").is_some() {
            println!("{}", render(&mut app, 130));
        }
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.focus, Pane::Articles);
        press(&mut app, KeyCode::Char('m'));
        assert_eq!(app.unread.get("f"), Some(&2), "m marks it unread again");
    }

    fn app<'a>(runtime: &'a Runtime, dir: &std::path::Path) -> App<'a> {
        let mut app = App::new(runtime, account(runtime, dir), HttpFetcher::new().unwrap());
        app.reload();
        app
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    #[test]
    fn the_add_dialog_takes_typing_until_closed() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "example.org/quiet");
        press(&mut app, KeyCode::Backspace);
        let screen = render(&mut app, 100);
        if std::env::var_os("MARINER_PRINT_UI").is_some() {
            println!("{screen}");
        }
        assert!(
            screen.contains(" Add feed ") && screen.contains("URL: example.org/quie"),
            "{screen}"
        );
        assert!(!app.quit, "q in the URL doesn't quit");

        press(&mut app, KeyCode::Esc);
        assert!(app.dialog.is_none());
        assert!(!render(&mut app, 100).contains(" Add feed "));
    }

    /// Ticks until background work (finding, subscribing) is done.
    fn settle(app: &mut App) {
        for _ in 0..500 {
            app.tick();
            if app.finding.is_none() && app.subscribing.is_none() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("background work didn't finish");
    }

    fn choice(url: &str, title: &str) -> FeedChoice {
        FeedChoice {
            url: url.into(),
            title: Some(title.into()),
        }
    }

    /// The seeded feed's URL: subscribing to it again needs no network.
    const SEEDED: &str = "https://example.com/feed.xml";

    #[test]
    fn always_shows_the_feeds_found_to_pick_from() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());

        app.dialog = Some(Dialog::Finding {
            url: "https://example.org".into(),
        });
        app.found(Ok(Vec::new()));
        let screen = render(&mut app, 100);
        assert!(
            screen.contains("No feed found there.") && screen.contains("URL: https://example.org"),
            "{screen}"
        );

        app.dialog = Some(Dialog::Finding {
            url: "https://example.org".into(),
        });
        app.found(Ok(vec![choice("https://example.org/posts.xml", "Posts")]));
        let screen = render(&mut app, 100);
        if std::env::var_os("MARINER_PRINT_UI").is_some() {
            println!("{screen}");
        }
        assert!(
            screen.contains("Posts")
                && screen.contains("[ Add to Folder… ]")
                && screen.contains("[ Subscribe ]"),
            "one feed still shows the selector: {screen}"
        );
    }

    #[test]
    fn subscribes_then_adds_to_the_folders_checked() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        app.dialog = Some(Dialog::choose(
            vec![choice(SEEDED, "Example Feed")],
            Vec::new(),
        ));

        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Enter);
        let screen = render(&mut app, 100);
        assert!(
            screen.contains(" Add to Folder ") && screen.contains("New folder…"),
            "{screen}"
        );

        press(&mut app, KeyCode::Char(' '));
        type_text(&mut app, "News");
        press(&mut app, KeyCode::Enter);
        let screen = render(&mut app, 100);
        if std::env::var_os("MARINER_PRINT_UI").is_some() {
            println!("{screen}");
        }
        assert!(screen.contains("[x] News"), "{screen}");

        press(&mut app, KeyCode::Esc);
        assert!(
            matches!(app.dialog, Some(Dialog::Choose { folders: None, .. })),
            "Esc goes back to the feeds"
        );
        press(&mut app, KeyCode::Enter);
        let Some(Dialog::Choose {
            folders: Some(picker),
            ..
        }) = &mut app.dialog
        else {
            panic!("Add to Folder is still focused");
        };
        picker.folders.push(("News".into(), true));
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.dialog, Some(Dialog::Subscribing { .. })));

        settle(&mut app);
        assert!(app.dialog.is_none());
        assert_eq!(app.status, "Subscribed to Example Feed");
        assert_eq!(app.feeds[0].tag_names, ["News"]);
        assert!(app.expanded.contains("News"), "the folder opens");
        assert_eq!(
            app.selected_row(),
            &Row::Feed {
                id: "f".into(),
                folder: Some("News".into())
            }
        );
    }

    #[test]
    fn subscribe_without_folders_and_by_clicking() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        app.dialog = Some(Dialog::choose(
            vec![choice(SEEDED, "Example Feed")],
            Vec::new(),
        ));

        render(&mut app, 100);
        let (area, _) = *app
            .buttons
            .iter()
            .find(|(_, button)| *button == Button::Subscribe)
            .expect("a Subscribe button");
        click(&mut app, area.x + 1, area.y);
        assert!(matches!(app.dialog, Some(Dialog::Subscribing { .. })));

        settle(&mut app);
        assert_eq!(app.status, "Subscribed to Example Feed");
        assert!(app.feeds[0].tag_names.is_empty(), "no folders");
    }

    /// Makes `app` record the URLs it opens instead of opening them.
    fn record_opens(app: &mut App) -> Arc<std::sync::Mutex<Vec<String>>> {
        let opened = Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = Arc::clone(&opened);
        app.opener = Box::new(move |url| {
            record.lock().unwrap().push(url.to_string());
            Ok(())
        });
        opened
    }

    fn mouse(app: &mut App, kind: MouseEventKind, column: u16, row: u16) {
        app.on_mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        });
    }

    fn click(app: &mut App, column: u16, row: u16) {
        mouse(app, MouseEventKind::Down(MouseButton::Left), column, row);
    }

    #[test]
    fn clicks_select_feeds_open_articles_and_follow_links() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        let opened = record_opens(&mut app);

        render(&mut app, 140);
        let feeds = app.areas.feeds.unwrap();
        click(&mut app, feeds.x + 3, feeds.y + 2);
        assert_eq!(
            app.feed_list.selected(),
            Some(1),
            "the second row is the feed"
        );

        render(&mut app, 140);
        let articles = app.areas.articles.unwrap();
        click(&mut app, articles.x + 3, articles.y + 3);
        assert_eq!(app.focus, Pane::Reader);
        let title = app.reader.as_ref().unwrap().article.title.clone();
        assert_eq!(
            title.as_deref(),
            Some("Second Post"),
            "the second article takes rows 3-4"
        );

        app.article_list.select(Some(0));
        app.open_article();
        render(&mut app, 140);
        let reader = app.reader.as_ref().unwrap();
        let (line, link) = reader
            .links
            .iter()
            .enumerate()
            .find_map(|(line, links)| {
                links
                    .iter()
                    .find(|link| &*link.url == "https://example.com/more")
                    .map(|link| (line as u16, link.clone()))
            })
            .unwrap();
        let area = app.areas.reader.unwrap();
        click(&mut app, area.x + 1 + link.start, area.y + 1 + line);
        click(
            &mut app,
            area.x + 1 + link.start.saturating_sub(2),
            area.y + 1 + line,
        );
        assert_eq!(
            *opened.lock().unwrap(),
            ["https://example.com/more"],
            "only the link opens"
        );
    }

    #[test]
    fn the_reader_stops_scrolling_at_the_end() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        app.open_article();
        render(&mut app, 80);
        for _ in 0..20 {
            press(&mut app, KeyCode::Char('j'));
        }
        let area = app.areas.reader.unwrap();
        mouse(&mut app, MouseEventKind::ScrollDown, area.x + 2, area.y + 2);
        let reader = app.reader.as_ref().unwrap();
        assert_eq!(reader.max_scroll, 0, "the short article fits");
        assert_eq!(reader.scroll, 0);

        // Three rows tall, the same article has to scroll, but only so far.
        let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 6)).unwrap();
        terminal.draw(|frame| ui::draw(frame, &mut app)).unwrap();
        for _ in 0..50 {
            press(&mut app, KeyCode::Char('j'));
        }
        let reader = app.reader.as_ref().unwrap();
        assert!(reader.max_scroll > 0);
        assert_eq!(reader.scroll, reader.max_scroll);
    }

    #[test]
    fn lists_the_articles_links() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        let opened = record_opens(&mut app);
        app.open_article();

        press(&mut app, KeyCode::Char('L'));
        let screen = render(&mut app, 100);
        assert!(
            screen.contains(" Links ") && screen.contains("more  https://example.com/more"),
            "{screen}"
        );
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Enter);
        assert!(app.dialog.is_none());
        assert_eq!(*opened.lock().unwrap(), ["https://example.com/more"]);
    }

    #[test]
    fn switches_between_the_feeds_content_and_the_full_text() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        app.open_article();
        let id = app.reader.as_ref().unwrap().article.id.clone();

        app.reader.as_mut().unwrap().wants_full_text = true;
        app.extracted(
            id.clone(),
            Ok(Some("<p>The whole story, at length.</p>".into())),
        );
        let screen = render(&mut app, 100);
        assert!(
            screen.contains("Article · full text") && screen.contains("The whole story"),
            "{screen}"
        );

        press(&mut app, KeyCode::Char('c'));
        let screen = render(&mut app, 100);
        assert!(
            screen.contains("Hello world") && !screen.contains("full text ─"),
            "{screen}"
        );

        press(&mut app, KeyCode::Char('c'));
        assert!(
            app.extracting.is_empty(),
            "the full text is kept, not fetched again"
        );
        assert!(render(&mut app, 100).contains("The whole story"));
    }

    #[test]
    fn shows_why_the_full_text_isnt_showing() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        app.open_article();
        let id = app.reader.as_ref().unwrap().article.id.clone();

        app.extracted(id.clone(), Ok(None));
        assert!(
            !render(&mut app, 100).contains("no full text"),
            "nothing to say when it wasn't asked for"
        );

        app.reader.as_mut().unwrap().wants_full_text = true;
        app.extracted(id.clone(), Err("timed out".into()));
        let screen = render(&mut app, 100);
        assert!(
            screen.contains("Article · full text failed") && screen.contains("Hello world"),
            "{screen}"
        );
        assert_eq!(
            app.status,
            "Couldn't get the full text: timed out; c to try again"
        );
    }

    #[test]
    fn remembers_full_text_for_the_feed() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        for (id, html) in [("a", "<p>The whole story.</p>"), ("b", "<p>All of it.</p>")] {
            app.full_texts
                .insert(id.into(), FullText::Found(html.into()));
        }

        app.open_article();
        press(&mut app, KeyCode::Char('c'));
        app.reload();
        assert!(app.feeds[0].sticky_full_content, "it's stored");

        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Enter);
        let screen = render(&mut app, 100);
        assert!(
            screen.contains("Article · full text") && screen.contains("All of it."),
            "{screen}"
        );

        press(&mut app, KeyCode::Char('c'));
        assert!(render(&mut app, 100).contains("Bye."));
        assert!(
            !app.feeds[0].sticky_full_content,
            "switching back forgets it"
        );
    }

    #[test]
    fn moves_a_feed_into_several_folders_and_out_of_all() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        let feed = app.feeds[0].clone();
        runtime
            .block_on(
                app.account
                    .update_feed(&feed, &feed.title, &["News".to_string()]),
            )
            .unwrap();
        app.set_folder_expanded("News", true);
        app.reload();
        app.select_feed("f", Some("News"));

        press(&mut app, KeyCode::Char('M'));
        let screen = render(&mut app, 120);
        assert!(
            screen.contains(" Move to ")
                && screen.contains("[x] News")
                && screen.contains("[ Move ]"),
            "its folder starts checked: {screen}"
        );
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char(' '));
        type_text(&mut app, "Tech");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.feeds[0].tag_names, ["News", "Tech"]);
        assert_eq!(app.status, "Moved Example Feed to News, Tech");

        press(&mut app, KeyCode::Char('M'));
        press(&mut app, KeyCode::Char(' '));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char(' '));
        press(&mut app, KeyCode::Enter);
        assert!(app.feeds[0].tag_names.is_empty());
        assert!(
            app.folder_names().is_empty(),
            "the folders it left empty are gone"
        );
    }

    #[test]
    fn renames_moves_and_removes_feeds() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        press(&mut app, KeyCode::Char('j'));

        press(&mut app, KeyCode::Char('e'));
        for _ in 0.."Feed".len() {
            press(&mut app, KeyCode::Backspace);
        }
        type_text(&mut app, "Site");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.feeds[0].title, "Example Site");

        press(&mut app, KeyCode::Char('M'));
        press(&mut app, KeyCode::Char(' '));
        type_text(&mut app, "News");
        press(&mut app, KeyCode::Enter);
        assert!(render(&mut app, 140).contains("[x] News"));
        press(&mut app, KeyCode::Enter);
        let screen = render(&mut app, 140);
        assert!(screen.contains("▾ News 2"), "{screen}");
        assert!(screen.contains("  Example Site 2"), "{screen}");
        assert_eq!(
            app.selected_row(),
            &Row::Feed {
                id: "f".into(),
                folder: Some("News".into())
            },
            "the moved feed stays selected"
        );

        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.articles.len(), 2, "a folder lists its feeds' articles");

        press(&mut app, KeyCode::Char('z'));
        let screen = render(&mut app, 140);
        assert!(
            screen.contains("▸ News 2") && !screen.contains("Example Site 2"),
            "{screen}"
        );
        press(&mut app, KeyCode::Char('z'));
        assert!(render(&mut app, 140).contains("▾ News 2"));

        press(&mut app, KeyCode::Char('d'));
        assert!(render(&mut app, 140).contains("Its feeds stay"));
        press(&mut app, KeyCode::Char('y'));
        assert_eq!(app.rows.len(), 2, "the feed is out of the folder");

        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('n'));
        assert_eq!(app.feeds.len(), 1, "n keeps it");
        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Enter);
        assert!(app.feeds.is_empty());
        assert_eq!(app.status, "Removed Example Site");
    }

    #[test]
    fn shows_condenses_output_for_whichever_content_is_open() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());
        app.open_article();

        press(&mut app, KeyCode::Char('/'));
        let screen = render(&mut app, 120);
        assert!(screen.contains("Article · condense"), "{screen}");
        assert!(screen.contains(r#""blocks": ["#), "{screen}");
        assert!(
            screen.contains(r#""text": "Hello world, see more.""#),
            "{screen}"
        );

        let id = app.reader.as_ref().unwrap().article.id.clone();
        app.reader.as_mut().unwrap().wants_full_text = true;
        app.extracted(id, Ok(Some("<p>The whole story.</p>".into())));
        let screen = render(&mut app, 120);
        assert!(
            screen.contains("Article · full text · condense"),
            "{screen}"
        );
        assert!(screen.contains(r#""text": "The whole story.""#), "{screen}");

        press(&mut app, KeyCode::Char('/'));
        let screen = render(&mut app, 120);
        assert!(
            screen.contains("The whole story.") && !screen.contains(r#""blocks""#),
            "{screen}"
        );
    }

    fn tempfile_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }
}
