//! The interactive reader: state, keys, and the event loop.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mariner_core::{AddFeedResult, Article, ArticleFilter, Feed};
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use crate::dialog::{Action, Dialog};
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
    let images = Images::new(mariner_feeds::fetch::HttpFetcher::new()?);
    let mut app = App::new(runtime, account, images);
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
    /// Row 0 is every feed; row n is `feeds[n - 1]`.
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
    pub status: String,
    /// A dialog over the panes, which takes every key while it's open.
    pub dialog: Option<Dialog>,
    /// Opens a URL in the browser; swappable so tests don't.
    opener: Box<dyn Fn(&str) -> std::io::Result<()>>,
    refreshing: Option<JoinHandle<mariner_core::Result<()>>>,
    adding: Option<JoinHandle<mariner_core::Result<AddFeedResult>>>,
    quit: bool,
}

/// Where the panes were last drawn, borders included.
#[derive(Debug, Default, Clone, Copy)]
pub struct Areas {
    pub feeds: Option<Rect>,
    pub articles: Option<Rect>,
    pub reader: Option<Rect>,
}

/// The open article.
pub struct Reader {
    pub article: Article,
    pub parts: Vec<Part>,
    pub scroll: u16,
    /// The furthest the article scrolls at the last drawn size.
    pub max_scroll: u16,
    /// The links on each line as last laid out.
    pub links: Vec<Vec<LinkRange>>,
}

impl<'a> App<'a> {
    fn new(runtime: &'a Runtime, account: Arc<LocalAccount>, images: Images) -> Self {
        App {
            runtime,
            account,
            feeds: Vec::new(),
            unread: HashMap::new(),
            feed_list: ListState::default().with_selected(Some(0)),
            filter: ArticleFilter::Unread,
            articles: Vec::new(),
            article_list: ListState::default(),
            reader: None,
            images,
            true_color: halfblock::true_color_terminal(),
            focus: Pane::Feeds,
            areas: Areas::default(),
            status: String::new(),
            dialog: None,
            opener: Box::new(open_in_system_browser),
            refreshing: None,
            adding: None,
            quit: false,
        }
    }

    pub fn is_refreshing(&self) -> bool {
        self.refreshing.is_some()
    }

    /// The selected feed, or `None` for every feed.
    pub fn selected_feed(&self) -> Option<&Feed> {
        let row = self.feed_list.selected().unwrap_or(0);
        row.checked_sub(1).and_then(|index| self.feeds.get(index))
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
            match dialog.on_key(key) {
                Action::None => {}
                Action::Close => self.dialog = None,
                Action::Add(url) => self.add_feed(url),
                Action::Open(url) => {
                    self.dialog = None;
                    self.open_url(&url);
                }
            }
            return;
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
            _ => {}
        }
    }

    /// Notices a finished refresh or add.
    fn tick(&mut self) {
        if self.adding.as_ref().is_some_and(JoinHandle::is_finished) {
            let handle = self.adding.take().expect("checked");
            let result = match self.runtime.block_on(handle) {
                Ok(result) => result.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            self.added(result);
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

    /// Finds and subscribes to the feeds at `url` in the background.
    fn add_feed(&mut self, url: String) {
        let account = Arc::clone(&self.account);
        let task_url = url.clone();
        self.adding = Some(
            self.runtime
                .spawn(async move { account.add_feed(&task_url, None, None).await }),
        );
        self.dialog = Some(Dialog::Adding { url });
    }

    /// Shows what adding a feed led to: the new feed selected, a list to
    /// choose from, or the URL again with what went wrong.
    fn added(&mut self, result: Result<AddFeedResult, String>) {
        let url = match &self.dialog {
            Some(Dialog::Adding { url }) => Some(url.clone()),
            _ => None,
        };
        let retry = |message: &str| Dialog::Add {
            input: url.clone().unwrap_or_default(),
            message: Some(message.to_string()),
        };
        match result {
            Ok(AddFeedResult::Added(feed)) => {
                self.dialog = None;
                self.status = format!("Added {}", feed.title);
                self.reload();
                if let Some(index) = self.feeds.iter().position(|listed| listed.id == feed.id) {
                    self.feed_list.select(Some(index + 1));
                    self.article_list.select(None);
                    self.load_articles();
                }
                self.focus = Pane::Feeds;
            }
            Ok(AddFeedResult::Choices(choices)) => self.dialog = Some(Dialog::choose(choices)),
            Ok(AddFeedResult::NotFound) => self.dialog = Some(retry("No feed found there.")),
            Err(error) => self.dialog = Some(retry(&error)),
        }
        // A dialog closed while adding reports in the status line instead.
        if url.is_none() && self.dialog.is_some() {
            self.dialog = None;
            self.status = "Couldn't add the feed".into();
        }
    }

    /// Reloads feeds, counts and the article list, keeping selections.
    fn reload(&mut self) {
        match self.runtime.block_on(self.account.feeds()) {
            Ok(feeds) => self.feeds = feeds,
            Err(error) => self.status = error.to_string(),
        }
        if let Ok(unread) = self.runtime.block_on(self.account.unread_counts()) {
            self.unread = unread;
        }
        let rows = self.feeds.len() + 1;
        if self.feed_list.selected().is_none_or(|row| row >= rows) {
            self.feed_list.select(Some(0));
        }
        self.load_articles();
    }

    fn load_articles(&mut self) {
        let feed_id = self.selected_feed().map(|feed| feed.id.clone());
        let selected_id = self
            .article_list
            .selected()
            .and_then(|row| self.articles.get(row))
            .map(|article| article.id.clone());
        let articles =
            self.runtime.block_on(
                self.account
                    .articles(self.filter, feed_id.as_deref(), PAGE, 0),
            );
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
                let rows = self.feeds.len() + 1;
                let row = step(self.feed_list.selected().unwrap_or(0), delta, rows);
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
                self.reader = Some(Reader {
                    article: content.article,
                    parts,
                    scroll: 0,
                    max_scroll: 0,
                    links: Vec::new(),
                });
                self.focus = Pane::Reader;
                if !article.read {
                    self.set_read(&article.id, true);
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
        let feed_id = self.selected_feed().map(|feed| feed.id.clone());
        match self
            .runtime
            .block_on(self.account.mark_all_read(feed_id.as_deref()))
        {
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
        if self.dialog.is_some() {
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
                if index <= self.feeds.len() && Some(index) != self.feed_list.selected() {
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
        let images = Images::new(HttpFetcher::new().unwrap());
        let mut app = App::new(&runtime, account(&runtime, dir.path()), images);
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
        let images = Images::new(HttpFetcher::new().unwrap());
        let mut app = App::new(&runtime, account(&runtime, dir.path()), images);
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
        let images = Images::new(HttpFetcher::new().unwrap());
        let mut app = App::new(runtime, account(runtime, dir), images);
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

    #[test]
    fn shows_what_adding_led_to() {
        let runtime = Runtime::new().unwrap();
        let dir = tempfile_dir();
        let mut app = app(&runtime, dir.path());

        app.dialog = Some(Dialog::Adding {
            url: "https://example.org".into(),
        });
        app.added(Ok(AddFeedResult::NotFound));
        let screen = render(&mut app, 100);
        assert!(
            screen.contains("No feed found there.") && screen.contains("URL: https://example.org"),
            "{screen}"
        );

        app.dialog = Some(Dialog::Adding {
            url: "https://example.org".into(),
        });
        let choice = |url: &str, title: &str| mariner_core::FeedChoice {
            url: url.into(),
            title: Some(title.into()),
        };
        app.added(Ok(AddFeedResult::Choices(vec![
            choice("https://example.org/posts.xml", "Posts"),
            choice("https://example.org/comments.xml", "Comments"),
        ])));
        press(&mut app, KeyCode::Char('j'));
        let screen = render(&mut app, 100);
        assert!(
            screen.contains("Posts") && screen.contains("Comments"),
            "{screen}"
        );
        let Some(Dialog::Choose { list, .. }) = &app.dialog else {
            panic!("expected choices");
        };
        assert_eq!(list.selected(), Some(1));

        app.dialog = Some(Dialog::Adding {
            url: "https://example.com/feed.xml".into(),
        });
        let feed = app.feeds[0].clone();
        app.feed_list.select(Some(0));
        app.added(Ok(AddFeedResult::Added(feed)));
        assert!(app.dialog.is_none());
        assert_eq!(
            app.feed_list.selected(),
            Some(1),
            "the new feed is selected"
        );
        assert_eq!(app.status, "Added Example Feed");
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

    fn tempfile_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }
}
