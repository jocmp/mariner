import SwiftUI

/// Folders to check, any number of them, and a new one to name: for a new
/// feed (Add to Folder…) and for moving one (Move to…).
struct FolderPickerView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    let title: String
    let confirmTitle: String
    var initiallyChecked: [String] = []
    let confirm: ([String]) -> Void

    @State private var folders: [String] = []
    @State private var checked: Set<String> = []
    @State private var newFolder = ""

    var body: some View {
        NavigationStack {
            List {
                if !folders.isEmpty {
                    Section {
                        ForEach(folders, id: \.self) { folder in
                            Button {
                                toggle(folder)
                            } label: {
                                Label(folder, systemImage: checked.contains(folder) ? "checkmark.circle.fill" : "circle")
                            }
                            .tint(.primary)
                        }
                    }
                }
                Section("New Folder") {
                    HStack {
                        TextField("Name", text: $newFolder)
                            .onSubmit(addNewFolder)
                        Button("Add", action: addNewFolder)
                            .disabled(newFolder.trimmingCharacters(in: .whitespaces).isEmpty)
                    }
                }
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", role: .cancel) { dismiss() }
                }
            }
            .safeAreaInset(edge: .bottom) {
                Button(confirmTitle) {
                    addNewFolder()
                    confirm(folders.filter(checked.contains))
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .frame(maxWidth: .infinity)
                .padding()
            }
            .task {
                let names = await model.folderNames()
                folders = names + initiallyChecked.filter { !names.contains($0) }
                checked = Set(initiallyChecked)
            }
        }
    }

    private func toggle(_ folder: String) {
        if checked.contains(folder) {
            checked.remove(folder)
        } else {
            checked.insert(folder)
        }
    }

    /// Adds the typed folder to the list, checked.
    private func addNewFolder() {
        let name = newFolder.trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty else { return }
        if !folders.contains(name) {
            folders.append(name)
        }
        checked.insert(name)
        newFolder = ""
    }
}
