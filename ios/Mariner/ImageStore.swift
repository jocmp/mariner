import Foundation
import ImageIO
import Mariner
import Observation
import UIKit

/// Article images and feed icons: downloaded once (kept on disk by the
/// session's cache), decoded at the size they're shown, kept in memory, and
/// with each image's shape remembered so layout doesn't jump when it arrives.
@Observable
final class ImageStore {
    static let shared = ImageStore()

    /// Width over height, by URL, for images that have loaded.
    private(set) var aspectRatios: [URL: CGFloat] = [:]

    @ObservationIgnored private let images = NSCache<NSURL, UIImage>()
    @ObservationIgnored private var loads: [URL: Task<UIImage?, Never>] = [:]
    @ObservationIgnored private let session: URLSession = {
        let configuration = URLSessionConfiguration.default
        configuration.urlCache = URLCache(
            memoryCapacity: 16 * 1024 * 1024,
            diskCapacity: 256 * 1024 * 1024,
            directory: URL.cachesDirectory.appending(path: "images")
        )
        configuration.requestCachePolicy = .returnCacheDataElseLoad
        return URLSession(configuration: configuration)
    }()

    private init() {
        images.totalCostLimit = 64 * 1024 * 1024
    }

    /// The image, when it's in memory already.
    func cachedImage(_ url: URL) -> UIImage? {
        images.object(forKey: url as NSURL)
    }

    /// The image, decoded no wider than `maxPixelWidth`. Loads of the same
    /// URL share one download.
    func image(_ url: URL, maxPixelWidth: CGFloat) async -> UIImage? {
        if let image = cachedImage(url) {
            return image
        }
        if let load = loads[url] {
            return await load.value
        }
        let session = session
        let load = Task.detached(priority: .utility) { () -> UIImage? in
            guard let (data, _) = try? await session.data(from: url) else { return nil }
            return Self.decode(data, maxPixelWidth: maxPixelWidth)
        }
        loads[url] = load
        let image = await load.value
        loads[url] = nil
        if let image {
            images.setObject(image, forKey: url as NSURL, cost: Self.cost(of: image))
            if image.size.height > 0 {
                aspectRatios[url] = image.size.width / image.size.height
            }
        }
        return image
    }

    /// Starts loading `urls`, a few at a time, so they're ready, and their
    /// shapes known, by the time they're scrolled to.
    func prefetch(_ urls: [URL], maxPixelWidth: CGFloat) {
        let urls = urls.filter { cachedImage($0) == nil }
        guard !urls.isEmpty else { return }
        Task {
            await withTaskGroup(of: Void.self) { group in
                var pending = urls.makeIterator()
                for _ in 0..<4 {
                    guard let url = pending.next() else { break }
                    group.addTask { _ = await self.image(url, maxPixelWidth: maxPixelWidth) }
                }
                while await group.next() != nil {
                    guard let url = pending.next() else { continue }
                    group.addTask { _ = await self.image(url, maxPixelWidth: maxPixelWidth) }
                }
            }
        }
    }

    /// Decodes the first frame, scaled down to `maxPixelWidth` when wider.
    /// The image's size is in pixels, at scale 1.
    nonisolated private static func decode(_ data: Data, maxPixelWidth: CGFloat) -> UIImage? {
        let options = [kCGImageSourceShouldCache: false] as CFDictionary
        guard let source = CGImageSourceCreateWithData(data as CFData, options) else { return nil }
        let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any]
        let width = properties?[kCGImagePropertyPixelWidth] as? CGFloat ?? 0
        let height = properties?[kCGImagePropertyPixelHeight] as? CGFloat ?? 0
        let longest = max(width, height)
        let scale = width > maxPixelWidth && width > 0 ? maxPixelWidth / width : 1
        let thumbnail = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceShouldCacheImmediately: true,
            kCGImageSourceThumbnailMaxPixelSize: max(1, (longest * scale).rounded()),
        ] as CFDictionary
        guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, thumbnail) else {
            return nil
        }
        return UIImage(cgImage: image)
    }

    nonisolated private static func cost(of image: UIImage) -> Int {
        guard let cgImage = image.cgImage else { return 1 }
        return cgImage.bytesPerRow * cgImage.height
    }
}
