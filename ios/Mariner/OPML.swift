import SwiftUI
import UniformTypeIdentifiers

extension UTType {
    /// Declared in Info.plist.
    nonisolated static let opml = UTType(importedAs: "org.opml.opml", conformingTo: .xml)
}

/// An OPML export, for the system's save sheet.
nonisolated struct OPMLDocument: FileDocument {
    static let readableContentTypes: [UTType] = [.opml]
    var text: String

    init(text: String) {
        self.text = text
    }

    init(configuration: ReadConfiguration) throws {
        text = String(decoding: configuration.file.regularFileContents ?? Data(), as: UTF8.self)
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: Data(text.utf8))
    }
}
