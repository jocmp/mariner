import Mariner
import SwiftUI

/// An article drawn from condense's blocks: the feed's content, or the full
/// text from its page. Choosing the full text sticks for the feed.
struct ReaderView: View {
    @Environment(AppModel.self) private var model
    let articleID: String

    @State private var content: ArticleContent?
    @State private var feedDocument: ArticleDocument?
    @State private var fullTextDocument: ArticleDocument?
    /// Whether the full text was asked for, with the button or by the
    /// feed's setting.
    @State private var wantsFullText = false
    @State private var isExtracting = false
    @State private var fullTextFailure: String?

    var body: some View {
        ScrollView {
            if let content {
                VStack(alignment: .leading, spacing: 16) {
                    header(content.article)
                    if wantsFullText, let fullTextFailure {
                        Label(fullTextFailure, systemImage: "exclamationmark.triangle")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                    }
                    if let document = shownDocument {
                        BlocksView(blocks: document.blocks)
                    }
                }
                .padding()
                .frame(maxWidth: 720, alignment: .leading)
                .frame(maxWidth: .infinity)
            }
        }
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { toolbar }
        .task { await load() }
    }

    private var shownDocument: ArticleDocument? {
        wantsFullText ? fullTextDocument ?? feedDocument : feedDocument
    }

    private var article: Article? {
        guard let content else { return nil }
        return model.articles.first { $0.id == content.article.id } ?? content.article
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

    @ToolbarContentBuilder
    private var toolbar: some ToolbarContent {
        if let article {
            ToolbarItemGroup(placement: .primaryAction) {
                Button(article.starred ? "Unstar" : "Star", systemImage: article.starred ? "star.fill" : "star") {
                    Task { await model.setStarred(article.id, !article.starred) }
                }
                Button("Full Text", systemImage: wantsFullText ? "doc.plaintext.fill" : "doc.plaintext") {
                    Task { await toggleFullText() }
                }
                .symbolEffect(.pulse, isActive: isExtracting)
                if let url = article.url.flatMap(URL.init(string:)) {
                    ShareLink(item: url)
                }
            }
        }
    }

    private func load() async {
        guard let content = await model.article(id: articleID) else { return }
        self.content = content
        let article = content.article
        feedDocument = await condensed(content.contentHtml ?? article.summary, base: article.url)
        if !article.read {
            await model.setRead(article.id, true)
        }
        if model.feed(id: article.feedId)?.stickyFullContent == true {
            wantsFullText = true
            await loadFullText()
        }
    }

    /// Switches between the feed's content and the full text, and remembers
    /// the choice for the feed. Asking again after a failure tries again.
    private func toggleFullText() async {
        guard let article = content?.article else { return }
        if wantsFullText, fullTextDocument != nil || isExtracting {
            wantsFullText = false
            await model.setStickyFullText(feedID: article.feedId, false)
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
        } catch {
            fullTextFailure = "Couldn't get the full text: \(error)"
        }
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
