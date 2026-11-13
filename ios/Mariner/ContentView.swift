import Mariner
import SwiftUI

/// Feeds, articles and the reader: side by side where there's room,
/// stacked on iPhone.
struct ContentView: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        @Bindable var model = model
        NavigationSplitView {
            SidebarView()
        } content: {
            ArticleListView()
        } detail: {
            if let id = model.selectedArticleID {
                ReaderView(articleID: id)
                    .id(id)
            } else {
                ContentUnavailableView("No Article Selected", systemImage: "doc.text")
            }
        }
        .onChange(of: model.selection) {
            model.selectedArticleID = nil
            Task { await model.loadArticles() }
        }
        // Switching the filter, like the selection, closes the article.
        .onChange(of: model.filter) {
            model.selectedArticleID = nil
            Task { await model.loadArticles() }
        }
        .alert(
            "Something went wrong",
            isPresented: Binding(
                get: { model.errorMessage != nil },
                set: { if !$0 { model.errorMessage = nil } }
            )
        ) {
            Button("OK") {}
        } message: {
            Text(model.errorMessage ?? "")
        }
    }
}
