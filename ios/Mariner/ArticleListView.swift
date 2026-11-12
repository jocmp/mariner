import Mariner
import SwiftUI

struct ArticleListView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        @Bindable var model = model
        List(model.articles, selection: $model.selectedArticleID) { article in
            ArticleRow(article: article, feed: model.feed(id: article.feedId))
                .swipeActions(edge: .leading) {
                    Button(article.read ? "Unread" : "Read", systemImage: article.read ? "circle.fill" : "circle") {
                        Task { await model.setRead(article.id, !article.read) }
                    }
                    .tint(.blue)
                }
                .swipeActions(edge: .trailing) {
                    Button(article.starred ? "Unstar" : "Star", systemImage: article.starred ? "star.slash" : "star") {
                        Task { await model.setStarred(article.id, !article.starred) }
                    }
                    .tint(.yellow)
                }
        }
        .listStyle(.plain)
        .navigationTitle(title)
        .refreshable { await model.refresh() }
        .overlay {
            if model.articles.isEmpty {
                ContentUnavailableView("Nothing Here", systemImage: "tray", description: Text(emptyMessage))
            }
        }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Menu("Filter", systemImage: "line.3.horizontal.decrease") {
                    Picker("Show", selection: $model.filter) {
                        Text("Unread").tag(ArticleFilter.unread)
                        Text("All").tag(ArticleFilter.all)
                        Text("Starred").tag(ArticleFilter.starred)
                    }
                    Divider()
                    Button("Mark All as Read", systemImage: "checkmark.circle") {
                        Task { await model.markAllRead() }
                    }
                }
            }
        }
    }

    private var title: String {
        switch model.selection ?? .all {
        case .all: "All Articles"
        case .folder(let name): name
        case .feed(let id): model.feed(id: id)?.title ?? "Feed"
        }
    }

    private var emptyMessage: String {
        switch model.filter {
        case .unread: "No unread articles. Pull to refresh."
        case .starred: "No starred articles."
        case .all: "No articles yet. Pull to refresh."
        }
    }
}

private struct ArticleRow: View {
    let article: Article
    let feed: Feed?

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Image(systemName: article.starred ? "star.fill" : "circle.fill")
                .font(.system(size: article.starred ? 10 : 8))
                .foregroundStyle(article.starred ? .yellow : .accentColor)
                .opacity(article.starred || !article.read ? 1 : 0)
            VStack(alignment: .leading, spacing: 4) {
                Text(article.title ?? "Untitled")
                    .font(.headline)
                    .foregroundStyle(article.read ? .secondary : .primary)
                    .lineLimit(3)
                Text([feed?.title, article.publishedAt.formatted(date: .abbreviated, time: .omitted)]
                    .compactMap(\.self)
                    .joined(separator: " · "))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            Spacer(minLength: 8)
            if let feed {
                FeedIcon(feed: feed)
                    .alignmentGuide(.firstTextBaseline) { $0[.bottom] - 3 }
            }
        }
        .padding(.vertical, 4)
    }
}

/// A feed's icon, or its first letter while there's none to show.
struct FeedIcon: View {
    let feed: Feed
    var size: CGFloat = 20

    var body: some View {
        AsyncImage(url: feed.faviconUrl.flatMap(URL.init(string:))) { phase in
            if case .success(let image) = phase {
                image
                    .resizable()
                    .scaledToFit()
            } else {
                Text(feed.title.prefix(1).uppercased())
                    .font(.system(size: size * 0.6, weight: .semibold))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(.fill.tertiary)
            }
        }
        .frame(width: size, height: size)
        .clipShape(.rect(cornerRadius: size * 0.22))
        .accessibilityHidden(true)
    }
}
