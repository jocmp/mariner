import ImageIO
import SwiftUI
import UIKit

/// A GIF, playing. SwiftUI's `Image` shows only one frame, so this is a
/// `UIImageView` with the frames decoded, scaled down, off the main thread.
struct AnimatedImage: UIViewRepresentable {
    let data: Data
    /// The widest each frame is decoded at, in pixels.
    let maxPixelWidth: CGFloat

    func makeUIView(context: Context) -> UIImageView {
        let view = UIImageView()
        view.contentMode = .scaleAspectFit
        view.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        view.setContentCompressionResistancePriority(.defaultLow, for: .vertical)
        let data = data
        let maxPixelWidth = maxPixelWidth
        Task {
            let image = await Task.detached(priority: .utility) {
                Self.animatedImage(data, maxPixelWidth: maxPixelWidth)
            }.value
            view.image = image
        }
        return view
    }

    func updateUIView(_ view: UIImageView, context: Context) {}

    /// Frames past this many are dropped, to bound memory.
    nonisolated private static let maxFrames = 300

    nonisolated private static func animatedImage(_ data: Data, maxPixelWidth: CGFloat) -> UIImage? {
        guard let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
        let count = min(CGImageSourceGetCount(source), maxFrames)
        let options = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceThumbnailMaxPixelSize: max(1, maxPixelWidth.rounded()),
        ] as CFDictionary
        var frames: [UIImage] = []
        var duration: TimeInterval = 0
        for index in 0..<count {
            guard let frame = CGImageSourceCreateThumbnailAtIndex(source, index, options) else { continue }
            frames.append(UIImage(cgImage: frame))
            duration += delay(source, at: index)
        }
        if frames.count < 2 {
            return frames.first
        }
        return UIImage.animatedImage(with: frames, duration: duration)
    }

    /// How long a frame shows, as browsers read it: under 0.02 s means 0.1 s.
    nonisolated private static func delay(_ source: CGImageSource, at index: Int) -> TimeInterval {
        let properties = CGImageSourceCopyPropertiesAtIndex(source, index, nil) as? [CFString: Any]
        let gif = properties?[kCGImagePropertyGIFDictionary] as? [CFString: Any]
        let delay = (gif?[kCGImagePropertyGIFUnclampedDelayTime] as? Double)
            ?? (gif?[kCGImagePropertyGIFDelayTime] as? Double)
            ?? 0.1
        return delay < 0.02 ? 0.1 : delay
    }
}
