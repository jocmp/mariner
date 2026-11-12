import Mariner
import SwiftUI

/// Every article, then folders with their feeds, then feeds in no folder.
/// Feeds and folders are renamed, moved and removed from their context
/// menus.
struct SidebarView: View {
    @Environment(AppModel.self) private var model
    @State private var isAddingFeed = false
    @State private var renaming: Target?
    @State private var newName = ""
    @State private var moving: Feed?
    @State private var removing: Target?
    @State private var isImporting = false
    @State private var exported: OPMLDocument?
    @State private var importMessage: String?

    /// A feed or a folder, to rename or remove.
    enum Target: Hashable {
        case feed(Feed)
        case folder(String)

        var name: String {
            switch self {
            case .feed(let feed): feed.title
            case .folder(let name): name
            }
        }
    }

    var body: some View {
        @Bindable var model = model
        List(selection: $model.selection) {
            Label("All Articles", systemImage: "tray")
                .badge(model.unreadCount(.all))
                .tag(SidebarItem.all)
            ForEach(model.folders, id: \.name) { folder in
                DisclosureGroup(isExpanded: expanded(folder)) {
                    ForEach(model.feeds(in: folder.name)) { feed in
                        feedRow(feed)
                    }
                } label: {
                    Label(folder.name, systemImage: "folder")
                        .badge(model.unreadCount(.folder(folder.name)))
                        .tag(SidebarItem.folder(folder.name))
                        .contextMenu {
                            Button("Rename…", systemImage: "pencil") { rename(.folder(folder.name)) }
                            Button("Remove", systemImage: "trash", role: .destructive) {
                                removing = .folder(folder.name)
                            }
                        }
                }
            }
            ForEach(model.feedsWithoutFolder) { feed in
                feedRow(feed)
            }
        }
        .navigationTitle("Mariner")
        .refreshable { await model.refresh() }
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button("Add Feed", systemImage: "plus") { isAddingFeed = true }
            }
            ToolbarItem(placement: .secondaryAction) {
                Menu("More", systemImage: "ellipsis") {
                    // Only local accounts import; it's the only kind so far.
                    Button("Import OPML…", systemImage: "square.and.arrow.down") {
                        isImporting = true
                    }
                    Button("Export OPML…", systemImage: "square.and.arrow.up") {
                        Task { await export() }
                    }
                }
            }
        }
        .fileImporter(isPresented: $isImporting, allowedContentTypes: [.opml, .xml]) { result in
            Task { await importOPML(result) }
        }
        .fileExporter(
            isPresented: Binding(get: { exported != nil }, set: { if !$0 { exported = nil } }),
            document: exported,
            contentType: .opml,
            defaultFilename: "Mariner"
        ) { result in
            if case .failure(let error) = result {
                model.show(error)
            }
        }
        .alert(
            "Imported",
            isPresented: Binding(get: { importMessage != nil }, set: { if !$0 { importMessage = nil } })
        ) {
            Button("OK") {}
        } message: {
            Text(importMessage ?? "")
        }
        .sheet(isPresented: $isAddingFeed) {
            AddFeedView()
        }
        .sheet(item: $moving) { feed in
            FolderPickerView(
                title: "Move to",
                confirmTitle: "Move",
                initiallyChecked: feed.tagNames
            ) { folders in
                moving = nil
                Task { await model.moveFeed(id: feed.id, to: folders) }
            }
        }
        .alert(
            renaming.map { if case .folder = $0 { "Rename Folder" } else { "Rename Feed" } } ?? "",
            isPresented: Binding(get: { renaming != nil }, set: { if !$0 { renaming = nil } }),
            presenting: renaming
        ) { target in
            TextField("Name", text: $newName)
            Button("Cancel", role: .cancel) {}
            Button("Rename") { commitRename(target) }
                .disabled(newName.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .confirmationDialog(
            removing.map { "Remove \($0.name)?" } ?? "",
            isPresented: Binding(get: { removing != nil }, set: { if !$0 { removing = nil } }),
            titleVisibility: .visible,
            presenting: removing
        ) { target in
            Button("Remove", role: .destructive) { commitRemove(target) }
        } message: { target in
            switch target {
            case .feed:
                Text("Its articles go too, starred ones included.")
            case .folder:
                Text("Its feeds stay.")
            }
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

    private func feedRow(_ feed: Feed) -> some View {
        Label(feed.title, systemImage: "dot.radiowaves.up.forward")
            .badge(model.unreadCount(.feed(feed.id)))
            .tag(SidebarItem.feed(feed.id))
            .contextMenu {
                Button("Rename…", systemImage: "pencil") { rename(.feed(feed)) }
                Button("Move to…", systemImage: "folder") { moving = feed }
                Button("Remove", systemImage: "trash", role: .destructive) {
                    removing = .feed(feed)
                }
            }
            .swipeActions {
                Button("Remove", systemImage: "trash", role: .destructive) {
                    removing = .feed(feed)
                }
            }
    }

    private func rename(_ target: Target) {
        newName = target.name
        renaming = target
    }

    private func commitRename(_ target: Target) {
        let name = newName.trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty else { return }
        Task {
            switch target {
            case .feed(let feed): await model.renameFeed(id: feed.id, to: name)
            case .folder(let folder): await model.renameFolder(folder, to: name)
            }
        }
    }

    private func commitRemove(_ target: Target) {
        Task {
            switch target {
            case .feed(let feed): await model.removeFeed(id: feed.id)
            case .folder(let folder): await model.removeFolder(folder)
            }
        }
    }

    private func importOPML(_ result: Result<URL, Error>) async {
        do {
            let url = try result.get()
            let accessing = url.startAccessingSecurityScopedResource()
            defer {
                if accessing { url.stopAccessingSecurityScopedResource() }
            }
            let imported = try await model.importOPML(Data(contentsOf: url))
            importMessage = switch (imported.added, imported.existing) {
            case (0, 0): "There were no feeds in that file."
            case (let added, 0): "Subscribed to \(added) feeds. Refreshing…"
            case (let added, let existing):
                "Subscribed to \(added) feeds; \(existing) were already here. Refreshing…"
            }
        } catch {
            model.show(error)
        }
    }

    private func export() async {
        do {
            exported = OPMLDocument(text: try await model.exportOPML())
        } catch {
            model.show(error)
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
