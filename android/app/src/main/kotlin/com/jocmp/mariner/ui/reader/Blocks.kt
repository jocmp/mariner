package com.jocmp.mariner.ui.reader

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.GraphicEq
import androidx.compose.material.icons.filled.PlayCircle
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.painter.ColorPainter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.BaselineShift
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import coil3.compose.AsyncImage
import coil3.request.ImageRequest
import com.jocmp.mariner.ffi.AudioBlock
import com.jocmp.mariner.ffi.Block
import com.jocmp.mariner.ffi.CellKind
import com.jocmp.mariner.ffi.ImageBlock
import com.jocmp.mariner.ffi.ImageSource
import com.jocmp.mariner.ffi.TableBlock
import com.jocmp.mariner.ffi.TextBlock
import com.jocmp.mariner.ffi.TextKind
import com.jocmp.mariner.ffi.TextStyle
import com.jocmp.mariner.ffi.VideoBlock
import kotlin.math.abs

/** One of condense's blocks as Compose views. */
@Composable
fun BlockView(block: Block, modifier: Modifier = Modifier) {
  when (block) {
    is Block.Text -> TextBlockView(block.block, modifier)
    is Block.Image -> ImageBlockView(block.block, modifier)
    is Block.Video -> VideoBlockView(block.block, modifier)
    is Block.Audio -> AudioBlockView(block.block, modifier)
    is Block.ListItem ->
      Row(modifier, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
          block.block.number?.let { "$it." } ?: "•",
          style = MaterialTheme.typography.bodyLarge,
          color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Blocks(block.block.blocks, Modifier.weight(1f))
      }
    is Block.Quote ->
      Row(modifier.height(IntrinsicSize.Min)) {
        Box(
          Modifier.width(3.dp)
            .fillMaxHeight()
            .background(MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(2.dp))
        )
        Blocks(block.block.blocks, Modifier.weight(1f).padding(start = 14.dp))
      }
    is Block.Table -> TableBlockView(block.block, modifier)
  }
}

@Composable
private fun Blocks(blocks: List<Block>, modifier: Modifier = Modifier) {
  Column(modifier, verticalArrangement = Arrangement.spacedBy(10.dp)) { blocks.forEach { BlockView(it) } }
}

@Composable
private fun TextBlockView(block: TextBlock, modifier: Modifier) {
  val text = annotated(block)
  val level = headingLevel(block)
  val style =
    when (level) {
      1 -> MaterialTheme.typography.headlineSmall
      2 -> MaterialTheme.typography.titleLarge
      null -> MaterialTheme.typography.bodyLarge
      else -> MaterialTheme.typography.titleMedium
    }
  when (block.kind) {
    TextKind.PROSE ->
      Text(
        text,
        style = style,
        fontWeight = if (level != null) FontWeight.Bold else null,
        modifier = modifier.padding(top = if (level != null) 8.dp else 0.dp),
      )
    TextKind.PREFORMATTED,
    TextKind.CODE ->
      Box(
        modifier
          .clip(RoundedCornerShape(10.dp))
          .background(MaterialTheme.colorScheme.surfaceContainerHighest)
          .horizontalScroll(rememberScrollState())
          .padding(12.dp)
      ) {
        Text(text, style = MaterialTheme.typography.bodyMedium, fontFamily = FontFamily.Monospace)
      }
  }
}

/** The text with its spans, whose offsets are UTF-16, as Kotlin's are. */
@Composable
private fun annotated(block: TextBlock): AnnotatedString {
  val linkColor = MaterialTheme.colorScheme.primary
  return buildAnnotatedString {
    append(block.text)
    val length = block.text.length
    block.spans.forEach { span ->
      val start = span.start.toInt().coerceIn(0, length)
      val end = span.end.toInt().coerceIn(start, length)
      when (val style = span.style) {
        TextStyle.Bold -> addStyle(SpanStyle(fontWeight = FontWeight.Bold), start, end)
        TextStyle.Italic -> addStyle(SpanStyle(fontStyle = FontStyle.Italic), start, end)
        TextStyle.Underline -> addStyle(SpanStyle(textDecoration = TextDecoration.Underline), start, end)
        TextStyle.Strikethrough -> addStyle(SpanStyle(textDecoration = TextDecoration.LineThrough), start, end)
        TextStyle.Superscript -> addStyle(SpanStyle(baselineShift = BaselineShift.Superscript, fontSize = 0.75.em), start, end)
        TextStyle.Subscript -> addStyle(SpanStyle(baselineShift = BaselineShift.Subscript, fontSize = 0.75.em), start, end)
        TextStyle.Code,
        TextStyle.Monospace -> addStyle(SpanStyle(fontFamily = FontFamily.Monospace), start, end)
        is TextStyle.Link ->
          addLink(LinkAnnotation.Url(style.url, TextLinkStyles(SpanStyle(color = linkColor))), start, end)
        is TextStyle.Heading,
        is TextStyle.Font -> {}
      }
    }
  }
}

/** The heading level, when a heading spans the whole block. */
private fun headingLevel(block: TextBlock): Int? =
  block.spans.firstNotNullOfOrNull { span ->
    val style = span.style
    if (style is TextStyle.Heading && span.start == 0u && span.end.toInt() == block.text.length) {
      style.level.toInt()
    } else {
      null
    }
  }

/** Each image's shape once it has loaded, for placing it before it loads again. */
private val aspectRatios = mutableStateMapOf<String, Float>()

/**
 * An image at the column's full width and its own shape, which its box takes
 * before it loads: from the HTML's size, a shape remembered from an earlier
 * load, or a common photo shape.
 */
@Composable
private fun ImageBlockView(block: ImageBlock, modifier: Modifier) {
  val uriHandler = LocalUriHandler.current
  BoxWithConstraints(modifier) {
    val density = LocalDensity.current
    val source = bestSource(block.sources, maxWidth, density.density) ?: return@BoxWithConstraints
    val ratio = source.aspectRatio ?: aspectRatios[source.url] ?: 1.5f
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
      AsyncImage(
        model = ImageRequest.Builder(LocalContext.current).data(source.url).build(),
        contentDescription = block.caption?.text,
        placeholder = ColorPainter(MaterialTheme.colorScheme.surfaceVariant),
        contentScale = ContentScale.FillWidth,
        onSuccess = { state ->
          val image = state.result.image
          if (image.height > 0) aspectRatios[source.url] = image.width.toFloat() / image.height
        },
        modifier =
          Modifier.fillMaxWidth()
            .aspectRatio(ratio)
            .clip(RoundedCornerShape(8.dp))
            .clickable(enabled = block.link != null) { block.link?.let(uriHandler::openUri) },
      )
      block.caption?.let {
        Text(annotated(it), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
      }
    }
  }
}

/** Width over height, from the `width` and `height` attributes. */
private val ImageSource.aspectRatio: Float?
  get() {
    val width = width?.toFloat() ?: return null
    val height = height?.toFloat() ?: return null
    return if (width > 0 && height > 0) width / height else null
  }

/**
 * The source closest to the size it shows at: `width` on a screen of
 * `density` pixels per dp, judged by pixel density (`2x`), width in pixels
 * (`640w`) or `width` attribute; one that says none counts as 1x.
 */
private fun bestSource(sources: List<ImageSource>, width: Dp, density: Float): ImageSource? =
  sources.minByOrNull { source ->
    val relative =
      source.density?.let { it / density }
        ?: source.srcsetWidth?.let { it.toFloat() / (width.value * density) }
        ?: source.width?.let { it.toFloat() / width.value }
        ?: (1 / density)
    abs(relative - 1)
  }

@Composable
private fun VideoBlockView(block: VideoBlock, modifier: Modifier) {
  val uriHandler = LocalUriHandler.current
  val source = block.sources.firstOrNull() ?: return
  Box(
    modifier
      .aspectRatio(16f / 9)
      .clip(RoundedCornerShape(8.dp))
      .background(MaterialTheme.colorScheme.surfaceVariant)
      .clickable { uriHandler.openUri(source.link ?: source.url) },
    contentAlignment = Alignment.Center,
  ) {
    source.thumbnail?.let {
      AsyncImage(model = it, contentDescription = null, contentScale = ContentScale.Crop, modifier = Modifier.fillMaxWidth())
    }
    Icon(Icons.Default.PlayCircle, contentDescription = "Play", tint = Color.White, modifier = Modifier.size(56.dp))
  }
}

@Composable
private fun AudioBlockView(block: AudioBlock, modifier: Modifier) {
  val uriHandler = LocalUriHandler.current
  val source = block.sources.firstOrNull() ?: return
  TextButton(onClick = { uriHandler.openUri(source.url) }, modifier = modifier) {
    Icon(Icons.Default.GraphicEq, contentDescription = null)
    Text("Play Audio", modifier = Modifier.padding(start = 8.dp))
  }
}

/** A grid of cells, rows of cells at their column spans, scrolling sideways. */
@Composable
private fun TableBlockView(block: TableBlock, modifier: Modifier) {
  val cellWidth = 160.dp
  Column(modifier.horizontalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
    (0u until block.rows).forEach { row ->
      Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        var col = 0u
        while (col < block.cols) {
          val cell = block.cells.find { it.row == row && it.col == col }
          val span = (cell?.colSpan ?: 1u).coerceAtLeast(1u)
          Box(Modifier.width(cellWidth * span.toInt())) {
            if (cell != null) {
              Column {
                cell.blocks.forEach { inner ->
                  if (cell.kind == CellKind.HEADER && inner is Block.Text) {
                    Text(inner.block.text, fontWeight = FontWeight.SemiBold)
                  } else {
                    BlockView(inner)
                  }
                }
              }
            }
          }
          col += span
        }
      }
    }
  }
}
