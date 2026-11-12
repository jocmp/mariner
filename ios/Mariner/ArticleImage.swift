import Nuke
import NukeUI
import SwiftUI

/// An image in an article. Its box takes the image's shape before it loads,
/// from the HTML's size or the shape remembered from an earlier load, so
/// the article doesn't jump; it isn't stretched past its natural width; and
/// GIFs animate.
struct ArticleImage: View {
    let url: URL
    /// From the HTML's `width` and `height`, when it gave both.
    var knownAspectRatio: CGFloat?
    /// The widest it should show, in points, when known before it loads.
    var naturalWidth: CGFloat?

    @State private var failed = false
    private let aspectRatios = AspectRatios.shared

    init(url: URL, knownAspectRatio: CGFloat? = nil, naturalWidth: CGFloat? = nil) {
        self.url = url
        self.knownAspectRatio = knownAspectRatio
        self.naturalWidth = naturalWidth
    }

    var body: some View {
        if !failed {
            LazyImage(request: Images.article(url)) { state in
                if let container = state.imageContainer {
                    image(container)
                } else {
                    Rectangle()
                        .fill(.fill.tertiary)
                        .aspectRatio(placeholderAspectRatio, contentMode: .fit)
                        .frame(maxWidth: naturalWidth ?? .infinity)
                }
            }
            .onCompletion { result in
                switch result {
                case .success(let response):
                    aspectRatios.record(url, size: response.image.size)
                case .failure:
                    failed = true
                }
            }
            .clipShape(.rect(cornerRadius: 8))
        }
    }

    @ViewBuilder
    private func image(_ container: ImageContainer) -> some View {
        let size = container.image.size
        let ratio = size.height > 0 ? size.width / size.height : placeholderAspectRatio
        Group {
            if container.type == .gif, let data = container.data {
                AnimatedImage(data: data, maxPixelWidth: size.width * container.image.scale)
            } else {
                Image(uiImage: container.image).resizable()
            }
        }
        .aspectRatio(ratio, contentMode: .fit)
        .frame(maxWidth: naturalWidth ?? size.width)
    }

    /// The HTML's shape, else one remembered, else a common photo shape.
    private var placeholderAspectRatio: CGFloat {
        knownAspectRatio ?? aspectRatios[url] ?? 3 / 2
    }
}
