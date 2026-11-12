import Mariner
import SwiftUI

struct ArticleListView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        @Bindable var model = model
        List(model.articles, selection: $model.selectedArticleID) { article in
            ArticleRow(article: article, feedTitle: model.feed(id: article.feedId)?.title)
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
    let feedTitle: String?

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
                Text([feedTitle, article.publishedAt.formatted(date: .abbreviated, time: .omitted)]
                    .compactMap(\.self)
                    .joined(separator: " · "))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.vertical, 4)
    }
}
