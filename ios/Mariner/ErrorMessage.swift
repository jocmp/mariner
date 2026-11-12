import Foundation
import Mariner

/// What to tell the user about an error: the reason core gives, not the
/// error's debug description.
func userMessage(for error: Error) -> String {
    guard let error = error as? MarinerError else {
        return error.localizedDescription
    }
    return switch error {
    case .Database(let reason), .Network(let reason), .Extract(let reason),
         .Opml(let reason), .Condense(let reason):
        reason
    case .NoFeed(let url):
        "There's no feed at \(url)."
    case .Status(let url, let status):
        "\(url) answered with HTTP \(status)."
    }
}
