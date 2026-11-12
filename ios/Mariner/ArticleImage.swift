import SwiftUI

/// An image from the web, through `ImageStore`. Its box takes the image's
/// shape before it loads, from the HTML's size or the shape remembered from
/// an earlier load, so the article doesn't jump; and it isn't stretched past
/// its natural width.
struct ArticleImage: View {
    let url: URL
    /// From the HTML's `width` and `height`, when it gave both.
    var knownAspectRatio: CGFloat?
    /// The widest it should show, in points, when known before it loads.
    var naturalWidth: CGFloat?

    @Environment(\.displayScale) private var displayScale
    @State private var image: UIImage?
    @State private var failed = false
    private let store = ImageStore.shared

    init(url: URL, knownAspectRatio: CGFloat? = nil, naturalWidth: CGFloat? = nil) {
        self.url = url
        self.knownAspectRatio = knownAspectRatio
        self.naturalWidth = naturalWidth
        _image = State(initialValue: ImageStore.shared.cachedImage(url))
    }

    var body: some View {
        if !failed {
            Group {
                if let image {
                    Image(uiImage: image).resizable()
                } else {
                    Rectangle().fill(.fill.tertiary)
                }
            }
            .aspectRatio(aspectRatio, contentMode: .fit)
            .frame(maxWidth: naturalWidth ?? image?.size.width ?? .infinity)
            .clipShape(.rect(cornerRadius: 8))
            .task(id: url) {
                guard image == nil else { return }
                image = await store.image(url, maxPixelWidth: ReaderLayout.maxWidth * displayScale)
                failed = image == nil
            }
        }
    }

    /// The loaded image's shape, else the HTML's, else one remembered, else
    /// a common photo shape as a guess.
    private var aspectRatio: CGFloat {
        if let image, image.size.height > 0 {
            return image.size.width / image.size.height
        }
        return knownAspectRatio ?? store.aspectRatios[url] ?? 3 / 2
    }
}
