import Mariner
import SwiftUI

/// A URL box, then the feeds it leads to (always, even one) with
/// "Add to Folder…" and "Subscribe" beneath. Subscribing here adds the feed
/// to no folder.
struct AddFeedView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var url = ""
    @State private var choices: [FeedChoice] = []
    @State private var selectedURL: String?
    @State private var isWorking = false
    @State private var message: String?
    @State private var isChoosingFolders = false
    @FocusState private var isURLFocused: Bool

    var body: some View {
        NavigationStack {
            Group {
                if choices.isEmpty {
                    urlForm
                } else {
                    feedSelector
                }
            }
            .navigationTitle("Add Feed")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", role: .cancel) { dismiss() }
                }
                if choices.isEmpty {
                    ToolbarItem(placement: .confirmationAction) {
                        if isWorking {
                            ProgressView()
                        } else {
                            Button("Find") { find() }
                                .disabled(url.trimmingCharacters(in: .whitespaces).isEmpty)
                        }
                    }
                }
            }
        }
        .sheet(isPresented: $isChoosingFolders) {
            FolderPickerView(title: "Add to Folder", confirmTitle: "Subscribe") { folders in
                subscribe(to: folders)
            }
        }
    }

    private var urlForm: some View {
        Form {
            Section {
                TextField("Site or feed URL", text: $url)
                    .keyboardType(.URL)
                    .textContentType(.URL)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .focused($isURLFocused)
                    .onSubmit { find() }
            } footer: {
                if let message {
                    Text(message).foregroundStyle(.red)
                }
            }
        }
        .onAppear { isURLFocused = true }
    }

    private var feedSelector: some View {
        List(choices, id: \.url, selection: $selectedURL) { choice in
            VStack(alignment: .leading) {
                Text(choice.title ?? "Untitled")
                Text(choice.url)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .environment(\.editMode, .constant(.active))
        .safeAreaInset(edge: .bottom) {
            VStack(spacing: 10) {
                if let message {
                    Text(message).font(.footnote).foregroundStyle(.red)
                }
                Button("Add to Folder…") { isChoosingFolders = true }
                    .buttonStyle(.bordered)
                Button("Subscribe") { subscribe(to: []) }
                    .buttonStyle(.borderedProminent)
            }
            .controlSize(.large)
            .frame(maxWidth: .infinity)
            .disabled(selectedURL == nil || isWorking)
            .padding()
        }
    }

    private func find() {
        let url = url.trimmingCharacters(in: .whitespaces)
        guard !url.isEmpty, !isWorking else { return }
        isWorking = true
        message = nil
        Task {
            defer { isWorking = false }
            do {
                let found = try await model.findFeeds(url: url)
                if found.isEmpty {
                    message = "No feed found there."
                } else {
                    choices = found
                    selectedURL = found.first?.url
                }
            } catch {
                message = userMessage(for: error)
            }
        }
    }

    private func subscribe(to folders: [String]) {
        guard let selectedURL, !isWorking else { return }
        isWorking = true
        message = nil
        Task {
            defer { isWorking = false }
            do {
                try await model.subscribe(url: selectedURL, folders: folders)
                dismiss()
            } catch {
                isChoosingFolders = false
                message = userMessage(for: error)
            }
        }
    }
}
