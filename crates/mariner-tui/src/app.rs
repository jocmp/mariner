//! The interactive reader: state, keys, and the event loop.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mariner_core::{Article, ArticleFilter, Feed};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::text::Line;
use ratatui::widgets::ListState;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use crate::{LocalAccount, reader, ui};

/// Articles loaded into the list at once.
const PAGE: usize = 500;

/// Runs the reader until the user quits.
pub fn run(
    runtime: &Runtime,
    account: Arc<LocalAccount>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(runtime, account);
    app.reload();
    let mut terminal = ratatui::init();
    let result = (|| -> std::io::Result<()> {
        while !app.quit {
            terminal.draw(|frame| ui::draw(frame, &mut app))?;
            if event::poll(Duration::from_millis(200))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                app.on_key(key);
            }
            app.tick();
        }
        Ok(())
    })();
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
    pub focus: Pane,
    pub status: String,
    refreshing: Option<JoinHandle<mariner_core::Result<()>>>,
    quit: bool,
}

/// The open article.
pub struct Reader {
    pub article: Article,
    pub lines: Vec<Line<'static>>,
    pub scroll: u16,
}

impl<'a> App<'a> {
    fn new(runtime: &'a Runtime, account: Arc<LocalAccount>) -> Self {
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
            focus: Pane::Feeds,
            status: String::new(),
            refreshing: None,
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
        match key.code {
            KeyCode::Char('q') => self.quit = true,
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
            _ => {}
        }
    }

    /// Notices a finished refresh.
    fn tick(&mut self) {
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
                    reader.scroll = reader.scroll.saturating_add_signed(delta as i16);
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
                let lines = reader::lines(&content, self.feed_title(&article.feed_id));
                self.reader = Some(Reader {
                    article: content.article,
                    lines,
                    scroll: 0,
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
        let Some(url) = self
            .current_article()
            .and_then(|article| article.url.clone())
        else {
            return;
        };
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        if let Err(error) = std::process::Command::new(opener).arg(&url).spawn() {
            self.status = format!("Couldn't open {url}: {error}");
        }
    }
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
                                   '<p>Hello <b>world</b>.</p><ul><li>One</li></ul>', 2000000000, 2000000000),
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
        let mut app = App::new(&runtime, account(&runtime, dir.path()));
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
        let mut app = App::new(&runtime, account(&runtime, dir.path()));
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
            reader.contains("First Post") && reader.contains("Hello world."),
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

    fn tempfile_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }
}
