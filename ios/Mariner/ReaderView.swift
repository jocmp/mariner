import Mariner
import Nuke
import SwiftUI

/// An article drawn from condense's blocks: the feed's content, or the full
/// text from its page. Choosing the full text sticks for the feed, and a
/// feed that opens in full text shows its own content only when the full
/// text fails.
struct ReaderView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.displayScale) private var displayScale
    let articleID: String

    @State private var content: ArticleContent?
    @State private var feedDocument: ArticleDocument?
    @State private var fullTextDocument: ArticleDocument?
    /// Whether the full text was asked for, with the button or by the
    /// feed's setting.
    @State private var wantsFullText = false
    @State private var isExtracting = false
    @State private var fullTextFailure: String?
    /// The article column's width, for choosing which image sources to load.
    @State private var columnWidth = ReaderLayout.maxWidth
    @State private var prefetcher = ImagePrefetcher(maxConcurrentRequestCount: 4)

    var body: some View {
        ScrollView {
            if let article {
                VStack(alignment: .leading, spacing: 16) {
                    header(article)
                    if wantsFullText, let fullTextFailure {
                        Label(fullTextFailure, systemImage: "exclamationmark.triangle")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                    }
                    if let document = shownDocument {
                        BlocksView(blocks: document.blocks)
                    } else {
                        SlowLoadingIndicator()
                    }
                }
                .padding()
                .frame(maxWidth: ReaderLayout.maxWidth, alignment: .leading)
                .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { columnWidth = $0 }
                .frame(maxWidth: .infinity)
            }
        }
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { toolbar }
        .task { await load() }
        .onDisappear { prefetcher.stopPrefetching() }
    }

    private var shownDocument: ArticleDocument? {
        wantsFullText ? fullTextDocument ?? feedDocument : feedDocument
    }

    /// The article as the list has it, so the header and toolbar show at
    /// once, before its content loads.
    private var article: Article? {
        model.articles.first { $0.id == articleID } ?? content?.article
    }

    @ViewBuilder
    private func header(_ article: Article) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            if let feed = model.feed(id: article.feedId) {
                Text(feed.title)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.secondary)
            }
            Text(article.title ?? "Untitled")
                .font(.title.bold())
            Text([article.author, article.publishedAt.formatted(date: .long, time: .shortened)]
                .compactMap(\.self)
                .joined(separator: " · "))
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
    }

    /// The same buttons for every article, so they stay put moving from one
    /// to the next.
    @ToolbarContentBuilder
    private var toolbar: some ToolbarContent {
        ToolbarItemGroup(placement: .primaryAction) {
            let starred = article?.starred == true
            Button(starred ? "Unstar" : "Star", systemImage: starred ? "star.fill" : "star") {
                guard let article else { return }
                Task { await model.setStarred(article.id, !starred) }
            }
            Button("Full Text", systemImage: wantsFullText ? "doc.plaintext.fill" : "doc.plaintext") {
                Task { await toggleFullText() }
            }
            .symbolEffect(.pulse, isActive: isExtracting)
            .disabled(article?.url == nil)
            ShareLink(item: article?.url.flatMap(URL.init(string:)) ?? URL(string: "about:blank")!)
                .disabled(article?.url == nil)
        }
    }

    /// Loads the article and marks it read. A feed that opens in full text
    /// gets the full text, and its own content only when that fails.
    private func load() async {
        if let feedID = article?.feedId, model.feed(id: feedID)?.stickyFullContent == true {
            wantsFullText = true
        }
        guard let content = await model.article(id: articleID) else { return }
        self.content = content
        if !content.article.read {
            await model.setRead(content.article.id, true)
        }
        if wantsFullText {
            await loadFullText()
            if fullTextDocument == nil {
                await loadFeedContent()
            }
        } else {
            await loadFeedContent()
        }
    }

    /// Condenses the content the feed gave, unless it was already.
    private func loadFeedContent() async {
        guard let content, feedDocument == nil else { return }
        let article = content.article
        feedDocument = await condensed(content.contentHtml ?? article.summary, base: article.url)
        prefetchImages(feedDocument)
    }

    /// Switches between the feed's content and the full text, and remembers
    /// the choice for the feed. Asking again after a failure tries again.
    private func toggleFullText() async {
        guard let article = content?.article else { return }
        if wantsFullText, fullTextDocument != nil || isExtracting {
            wantsFullText = false
            await model.setStickyFullText(feedID: article.feedId, false)
            await loadFeedContent()
            return
        }
        wantsFullText = true
        await model.setStickyFullText(feedID: article.feedId, true)
        await loadFullText()
    }

    private func loadFullText() async {
        guard let article = content?.article, fullTextDocument == nil, !isExtracting else { return }
        isExtracting = true
        fullTextFailure = nil
        defer { isExtracting = false }
        do {
            guard let html = try await model.fullText(for: article) else {
                fullTextFailure = "No full text found for this article."
                return
            }
            fullTextDocument = await condensed(html, base: article.url)
            prefetchImages(fullTextDocument)
        } catch {
            fullTextFailure = "Couldn't get the full text: \(userMessage(for: error))"
        }
        if fullTextDocument == nil {
            await loadFeedContent()
        }
    }

    /// Starts loading the article's images, so they're ready, and their
    /// shapes known, before they're scrolled to.
    private func prefetchImages(_ document: ArticleDocument?) {
        guard let document else { return }
        let width = columnWidth
        let requests = document.blocks
            .flatMap(\.imageBlocks)
            .compactMap { $0.bestSource(width: width, scale: displayScale) }
            .compactMap { URL(string: $0.url) }
            .map(Images.article)
        prefetcher.startPrefetching(with: requests)
    }

    /// Condenses `html` off the main thread.
    private func condensed(_ html: String?, base: String?) async -> ArticleDocument? {
        guard let html else { return nil }
        let base = base ?? ""
        return await Task.detached(priority: .userInitiated) {
            try? condense(html: html, baseUrl: base)
        }.value
    }
}

/// A spinner that shows only once loading has taken a while: 3 seconds.
/// Before that the space stays empty, so a quick load doesn't flash one.
private struct SlowLoadingIndicator: View {
    @State private var isSlow = false

    var body: some View {
        ZStack {
            if isSlow {
                ProgressView()
                    .transition(.opacity)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 32)
        .task {
            try? await Task.sleep(for: .seconds(3))
            withAnimation { isSlow = true }
        }
    }
}
