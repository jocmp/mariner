import Foundation
import Nuke
import Observation

/// Article images and feed icons go through Nuke: downloaded once into a
/// disk cache, decoded off the main thread at the size they're shown, and
/// kept in memory.
enum Images {
    /// Sets up the shared pipeline, before any image loads.
    static func configure() {
        ImagePipeline.shared = ImagePipeline(
            configuration: .withDataCache(name: "images", sizeLimit: 256 * 1024 * 1024)
        )
    }

    /// An article image, decoded no wider than the reader's column.
    static func article(_ url: URL) -> ImageRequest {
        ImageRequest(url: url, processors: [.resize(width: ReaderLayout.maxWidth)])
    }

    /// A feed icon `size` points square.
    static func icon(_ url: URL, size: CGFloat) -> ImageRequest {
        ImageRequest(url: url, processors: [.resize(width: size)])
    }
}

/// Each image's shape once it has loaded, so a box sized for it before it
/// loads again doesn't change.
@Observable
final class AspectRatios {
    static let shared = AspectRatios()

    private var ratios: [URL: CGFloat] = [:]

    subscript(url: URL) -> CGFloat? {
        ratios[url]
    }

    func record(_ url: URL, size: CGSize) {
        guard size.width > 0, size.height > 0, ratios[url] == nil else { return }
        ratios[url] = size.width / size.height
    }
}
