import Mariner
import SwiftUI

/// A URL box; a list to pick from when the URL leads to several feeds.
struct AddFeedView: View {
    @Environment(AppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var url = ""
    @State private var isAdding = false
    @State private var choices: [FeedChoice] = []
    @State private var message: String?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Site or feed URL", text: $url)
                        .keyboardType(.URL)
                        .textContentType(.URL)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .onSubmit { add(url) }
                } footer: {
                    if let message {
                        Text(message).foregroundStyle(.red)
                    }
                }
                if !choices.isEmpty {
                    Section("Several feeds; pick one") {
                        ForEach(choices, id: \.url) { choice in
                            Button {
                                add(choice.url)
                            } label: {
                                VStack(alignment: .leading) {
                                    Text(choice.title ?? "Untitled")
                                    Text(choice.url)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                }
            }
            .navigationTitle("Add Feed")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", role: .cancel) { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    if isAdding {
                        ProgressView()
                    } else {
                        Button("Add", role: .confirm) { add(url) }
                            .disabled(url.trimmingCharacters(in: .whitespaces).isEmpty)
                    }
                }
            }
        }
    }

    private func add(_ url: String) {
        let url = url.trimmingCharacters(in: .whitespaces)
        guard !url.isEmpty, !isAdding else { return }
        isAdding = true
        message = nil
        Task {
            defer { isAdding = false }
            do {
                switch try await model.addFeed(url: url) {
                case .added:
                    dismiss()
                case .choices(let found):
                    choices = found
                case .notFound:
                    message = "No feed found there."
                }
            } catch {
                message = String(describing: error)
            }
        }
    }
}
