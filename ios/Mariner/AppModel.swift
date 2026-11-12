import Foundation
import Mariner
import Observation

/// What the sidebar selects.
enum SidebarItem: Hashable {
    case all
    case folder(String)
    case feed(String)

    var scope: ArticleScope {
        switch self {
        case .all: .all
        case .folder(let name): .tag(name: name)
        case .feed(let id): .feed(id: id)
        }
    }
}

/// The account and what the screens show from it.
@Observable
final class AppModel {
    private(set) var account: Account?
    private(set) var feeds: [Feed] = []
    private(set) var folders: [Tag] = []
    private(set) var unreadCounts: [String: UInt64] = [:]
    private(set) var articles: [Article] = []
    private(set) var isRefreshing = false
    var selection: SidebarItem? = .all
    var selectedArticleID: String?
    var filter: ArticleFilter = .unread
    var errorMessage: String?

    /// Full texts extracted this session, by article id.
    private var fullTexts: [String: String] = [:]

    /// Opens the local account in Application Support.
    func open() async {
        guard account == nil else { return }
        do {
            let path = try FileManager.default
                .url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
                .appending(path: "accounts/local.db")
                .path(percentEncoded: false)
            account = try await Account.local(path: path, http: URLSessionClient())
            await reload()
        } catch {
            show(error)
        }
    }

    /// Reloads feeds, folders, counts and the article list.
    func reload() async {
        guard let account else { return }
        do {
            feeds = try await account.feeds()
            let tagged = Set(feeds.flatMap(\.tagNames))
            folders = try await account.tags().filter { tagged.contains($0.name) }
            unreadCounts = try await account.unreadCounts()
        } catch {
            show(error)
        }
        await loadArticles()
    }

    func loadArticles() async {
        guard let account else { return }
        do {
            articles = try await account.articles(
                filter: filter,
                scope: (selection ?? .all).scope,
                limit: 500,
                offset: 0
            )
        } catch {
            show(error)
        }
    }

    func refresh() async {
        guard let account, !isRefreshing else { return }
        isRefreshing = true
        defer { isRefreshing = false }
        do {
            try await account.refresh(cutoff: nil)
        } catch {
            show(error)
        }
        await reload()
    }

    /// The feeds `url` leads to, to pick one from. Subscribes to nothing.
    func findFeeds(url: String) async throws -> [FeedChoice] {
        guard let account else { return [] }
        return try await account.findFeeds(url: url)
    }

    /// Every folder's name, for adding a new feed to.
    func folderNames() async -> [String] {
        (try? await account?.tags().map(\.name)) ?? []
    }

    /// Subscribes to the feed at `url`, then, as its own change, adds it to
    /// `folders`, opening them so the feed shows.
    func subscribe(url: String, folders: [String]) async throws {
        guard let account else { return }
        var feed = try await account.subscribe(url: url, title: nil)
        if !folders.isEmpty {
            let tags = Array(Set(feed.tagNames).union(folders)).sorted()
            feed = try await account.updateFeed(feed: feed, title: feed.title, tagNames: tags)
            for folder in folders {
                try await account.setTagExpanded(name: folder, expanded: true)
            }
        }
        await reload()
        selection = .feed(feed.id)
        await loadArticles()
    }

    func feed(id: String) -> Feed? {
        feeds.first { $0.id == id }
    }

    func feeds(in folder: String) -> [Feed] {
        feeds.filter { $0.tagNames.contains(folder) }
    }

    var feedsWithoutFolder: [Feed] {
        feeds.filter(\.tagNames.isEmpty)
    }

    func unreadCount(_ item: SidebarItem) -> Int {
        switch item {
        case .all: Int(unreadCounts.values.reduce(0, +))
        case .folder(let name): Int(feeds(in: name).compactMap { unreadCounts[$0.id] }.reduce(0, +))
        case .feed(let id): Int(unreadCounts[id] ?? 0)
        }
    }

    func setExpanded(_ folder: String, _ expanded: Bool) async {
        guard let account else { return }
        if let index = folders.firstIndex(where: { $0.name == folder }) {
            folders[index].expanded = expanded
        }
        do {
            try await account.setTagExpanded(name: folder, expanded: expanded)
        } catch {
            show(error)
        }
    }

    func article(id: String) async -> ArticleContent? {
        do {
            return try await account?.article(id: id)
        } catch {
            show(error)
            return nil
        }
    }

    /// Marks an article read or unread, updating the list in place so it
    /// doesn't jump while reading through unread articles.
    func setRead(_ id: String, _ read: Bool) async {
        guard let account else { return }
        do {
            if read {
                try await account.markRead(articleIds: [id])
            } else {
                try await account.markUnread(articleIds: [id])
            }
            updateListed(id) { $0.read = read }
            unreadCounts = try await account.unreadCounts()
        } catch {
            show(error)
        }
    }

    func setStarred(_ id: String, _ starred: Bool) async {
        guard let account else { return }
        do {
            if starred {
                try await account.addStar(articleIds: [id])
            } else {
                try await account.removeStar(articleIds: [id])
            }
            updateListed(id) { $0.starred = starred }
        } catch {
            show(error)
        }
    }

    func markAllRead() async {
        guard let account else { return }
        do {
            try await account.markAllRead(scope: (selection ?? .all).scope)
        } catch {
            show(error)
        }
        await reload()
    }

    /// The article's full text, extracted from its page once per session.
    func fullText(for article: Article) async throws -> String? {
        if let html = fullTexts[article.id] {
            return html
        }
        guard let account, let url = article.url else { return nil }
        let html = try await account.fullContent(url: url)
        fullTexts[article.id] = html
        return html
    }

    /// Whether the feed's articles open in full text.
    func setStickyFullText(feedID: String, _ enabled: Bool) async {
        guard let account, let index = feeds.firstIndex(where: { $0.id == feedID }),
              feeds[index].stickyFullContent != enabled
        else { return }
        feeds[index].stickyFullContent = enabled
        do {
            try await account.setStickyFullContent(feedId: feedID, enabled: enabled)
        } catch {
            show(error)
        }
    }

    func show(_ error: Error) {
        errorMessage = String(describing: error)
    }

    private func updateListed(_ id: String, _ change: (inout Article) -> Void) {
        if let index = articles.firstIndex(where: { $0.id == id }) {
            change(&articles[index])
        }
    }
}
