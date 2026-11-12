import Mariner
import SwiftUI

/// Every article, then folders with their feeds, then feeds in no folder.
struct SidebarView: View {
    @Environment(AppModel.self) private var model
    @State private var isAddingFeed = false

    var body: some View {
        @Bindable var model = model
        List(selection: $model.selection) {
            Label("All Articles", systemImage: "tray")
                .badge(model.unreadCount(.all))
                .tag(SidebarItem.all)
            ForEach(model.folders, id: \.name) { folder in
                DisclosureGroup(isExpanded: expanded(folder)) {
                    ForEach(model.feeds(in: folder.name)) { feed in
                        FeedRow(feed: feed)
                    }
                } label: {
                    Label(folder.name, systemImage: "folder")
                        .badge(model.unreadCount(.folder(folder.name)))
                        .tag(SidebarItem.folder(folder.name))
                }
            }
            ForEach(model.feedsWithoutFolder) { feed in
                FeedRow(feed: feed)
            }
        }
        .navigationTitle("Mariner")
        .refreshable { await model.refresh() }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button("Add Feed", systemImage: "plus") { isAddingFeed = true }
            }
        }
        .sheet(isPresented: $isAddingFeed) {
            AddFeedView()
        }
        .overlay {
            if model.feeds.isEmpty, model.account != nil {
                ContentUnavailableView {
                    Label("No Feeds", systemImage: "dot.radiowaves.up.forward")
                } description: {
                    Text("Add a site or feed to start reading.")
                } actions: {
                    Button("Add Feed") { isAddingFeed = true }
                        .buttonStyle(.borderedProminent)
                }
            }
        }
    }

    /// Opens and closes a folder, remembering it.
    private func expanded(_ folder: Tag) -> Binding<Bool> {
        Binding {
            folder.expanded
        } set: { expanded in
            Task { await model.setExpanded(folder.name, expanded) }
        }
    }
}

private struct FeedRow: View {
    @Environment(AppModel.self) private var model
    let feed: Feed

    var body: some View {
        Label(feed.title, systemImage: "dot.radiowaves.up.forward")
            .badge(model.unreadCount(.feed(feed.id)))
            .tag(SidebarItem.feed(feed.id))
    }
}
