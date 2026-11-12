import Mariner
import SwiftUI

/// Condense's blocks as native views.
struct BlocksView: View {
    let blocks: [Block]

    var body: some View {
        LazyVStack(alignment: .leading, spacing: 14) {
            ForEach(blocks.indices, id: \.self) { index in
                BlockView(block: blocks[index])
            }
        }
    }
}

struct BlockView: View {
    let block: Block

    var body: some View {
        switch block {
        case .text(let text):
            TextBlockView(block: text)
        case .image(let image):
            ImageBlockView(block: image)
        case .video(let video):
            VideoBlockView(block: video)
        case .audio(let audio):
            if let url = audio.sources.first.flatMap({ URL(string: $0.url) }) {
                Link(destination: url) {
                    Label("Play Audio", systemImage: "waveform")
                }
            }
        case .listItem(let item):
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(item.number.map { "\($0)." } ?? "•")
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
                BlocksView(blocks: item.blocks)
            }
        case .quote(let quote):
            BlocksView(blocks: quote.blocks)
                .padding(.leading, 14)
                .overlay(alignment: .leading) {
                    Capsule()
                        .fill(.tertiary)
                        .frame(width: 3)
                }
        case .table(let table):
            TableBlockView(block: table)
        }
    }
}

private struct TextBlockView: View {
    let block: TextBlock

    var body: some View {
        switch block.kind {
        case .prose:
            Text(block.attributedText)
                .textSelection(.enabled)
                .padding(.top, block.headingLevel == nil ? 0 : 8)
        case .preformatted, .code:
            ScrollView(.horizontal) {
                Text(block.attributedText)
                    .font(.system(.callout, design: .monospaced))
                    .textSelection(.enabled)
                    .padding(12)
            }
            .background(.fill.tertiary, in: .rect(cornerRadius: 10))
        }
    }
}

private struct ImageBlockView: View {
    @Environment(\.openURL) private var openURL
    @Environment(\.displayScale) private var displayScale
    let block: ImageBlock
    @State private var width: CGFloat = ReaderLayout.maxWidth

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let source = block.bestSource(width: width, scale: displayScale),
               let url = URL(string: source.url) {
                ArticleImage(
                    url: url,
                    knownAspectRatio: source.aspectRatio,
                    naturalWidth: source.naturalWidth
                )
                .frame(maxWidth: .infinity, alignment: .leading)
                .onTapGesture {
                    if let link = block.link.flatMap(URL.init(string:)) {
                        openURL(link)
                    }
                }
            }
            if let caption = block.caption {
                Text(caption.attributedText)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { width = $0 }
    }
}

private struct VideoBlockView: View {
    let block: VideoBlock

    var body: some View {
        if let source = block.sources.first,
           let url = URL(string: source.link ?? source.url) {
            Link(destination: url) {
                ZStack {
                    if let thumbnail = source.thumbnail.flatMap(URL.init(string:)) {
                        ArticleImage(url: thumbnail, knownAspectRatio: 16 / 9)
                    } else {
                        Rectangle().fill(.fill.tertiary)
                    }
                    Image(systemName: "play.circle.fill")
                        .font(.system(size: 48))
                        .foregroundStyle(.white, .black.opacity(0.4))
                }
                .aspectRatio(16 / 9, contentMode: .fit)
                .clipShape(.rect(cornerRadius: 8))
            }
        }
    }
}

/// A grid of cells; cells spanning rows leave their later rows blank.
private struct TableBlockView: View {
    let block: TableBlock

    var body: some View {
        ScrollView(.horizontal) {
            Grid(alignment: .topLeading, horizontalSpacing: 16, verticalSpacing: 10) {
                ForEach(0..<Int(block.rows), id: \.self) { row in
                    GridRow {
                        ForEach(slots(in: UInt32(row)), id: \.col) { slot in
                            Group {
                                if let cell = slot.cell {
                                    BlocksView(blocks: cell.blocks)
                                        .fontWeight(cell.kind == .header ? .semibold : nil)
                                } else {
                                    Color.clear.frame(width: 0, height: 0)
                                }
                            }
                            .gridCellColumns(Int(slot.span))
                        }
                    }
                    if row == 0 {
                        Divider()
                    }
                }
            }
            .padding(.vertical, 4)
        }
    }

    private struct Slot {
        let col: UInt32
        let span: UInt32
        let cell: TableCell?
    }

    /// The row's cells, with blanks where a cell from an earlier row spans
    /// down or no cell is.
    private func slots(in row: UInt32) -> [Slot] {
        var slots: [Slot] = []
        var col: UInt32 = 0
        while col < block.cols {
            if let cell = block.cells.first(where: { $0.row == row && $0.col == col }) {
                slots.append(Slot(col: col, span: max(cell.colSpan, 1), cell: cell))
                col += max(cell.colSpan, 1)
            } else {
                let above = block.cells.first { $0.col == col && $0.row < row && $0.row + $0.rowSpan > row }
                let span = max(above?.colSpan ?? 1, 1)
                slots.append(Slot(col: col, span: span, cell: nil))
                col += span
            }
        }
        return slots
    }
}

extension TextBlock {
    /// The text with its spans applied. Offsets are UTF-16, as `NSRange`
    /// counts.
    var attributedText: AttributedString {
        var result = AttributedString(text)
        if let level = headingLevel {
            result.font = Self.headingFont(level)
        }
        for span in spans {
            let utf16 = NSRange(location: Int(span.start), length: Int(span.end) - Int(span.start))
            guard let textRange = Range(utf16, in: text),
                  let range = Range(textRange, in: result)
            else { continue }
            switch span.style {
            case .bold:
                add(.stronglyEmphasized, to: range, in: &result)
            case .italic:
                add(.emphasized, to: range, in: &result)
            case .code, .monospace:
                add(.code, to: range, in: &result)
            case .strikethrough:
                result[range].strikethroughStyle = .single
            case .underline:
                result[range].underlineStyle = .single
            case .superscript:
                result[range].baselineOffset = 6
            case .subscript:
                result[range].baselineOffset = -3
            case .link(let url):
                result[range].link = URL(string: url)
            case .heading, .font:
                break
            }
        }
        return result
    }

    /// The heading level, when a heading spans the whole block.
    var headingLevel: UInt8? {
        let length = UInt32(text.utf16.count)
        for span in spans {
            if case .heading(let level) = span.style, span.start == 0, span.end == length {
                return level
            }
        }
        return nil
    }

    private static func headingFont(_ level: UInt8) -> Font {
        switch level {
        case 1: .title.bold()
        case 2: .title2.bold()
        case 3: .title3.bold()
        default: .headline
        }
    }

    /// Adds an intent to each run in `range`, keeping the ones it has.
    private func add(_ intent: InlinePresentationIntent, to range: Range<AttributedString.Index>, in string: inout AttributedString) {
        let runs = string[range].runs.map { ($0.range, $0.inlinePresentationIntent ?? []) }
        for (runRange, existing) in runs {
            string[runRange].inlinePresentationIntent = existing.union(intent)
        }
    }
}

/// The reader's measurements.
enum ReaderLayout {
    /// The widest the article's column gets, in points.
    static let maxWidth: CGFloat = 720
}

extension ImageBlock {
    /// The source closest to the size it shows at: `width` points on a
    /// screen of `scale` pixels per point. Sources say how big they are by
    /// pixel density (`2x`), width in pixels (`640w`), or `width`
    /// attribute; one that says none counts as 1x.
    func bestSource(width: CGFloat, scale: CGFloat) -> ImageSource? {
        sources.min { first, second in
            abs(first.relativeSize(width: width, scale: scale) - 1)
                < abs(second.relativeSize(width: width, scale: scale) - 1)
        }
    }
}

extension ImageSource {
    /// Width over height, from the `width` and `height` attributes.
    var aspectRatio: CGFloat? {
        guard let width, let height, width > 0, height > 0 else { return nil }
        return CGFloat(width) / CGFloat(height)
    }

    /// How wide it's meant to show, in points, from its `width` attribute,
    /// as a browser would: an image is never stretched past it.
    var naturalWidth: CGFloat? {
        width.map { CGFloat($0) }
    }

    /// Its size relative to what's needed: 1 is just right.
    fileprivate func relativeSize(width shown: CGFloat, scale: CGFloat) -> CGFloat {
        if let density {
            return CGFloat(density) / scale
        }
        if let srcsetWidth {
            return CGFloat(srcsetWidth) / (shown * scale)
        }
        if let width {
            return CGFloat(width) / shown
        }
        return 1 / scale
    }
}

extension Block {
    /// The image blocks in this block, nested ones included.
    var imageBlocks: [ImageBlock] {
        switch self {
        case .image(let image): [image]
        case .listItem(let item): item.blocks.flatMap(\.imageBlocks)
        case .quote(let quote): quote.blocks.flatMap(\.imageBlocks)
        case .table(let table): table.cells.flatMap { $0.blocks.flatMap(\.imageBlocks) }
        case .text, .video, .audio: []
        }
    }
}
