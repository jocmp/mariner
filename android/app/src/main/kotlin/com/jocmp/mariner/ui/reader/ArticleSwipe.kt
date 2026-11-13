package com.jocmp.mariner.ui.reader

import androidx.compose.animation.core.Spring
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.graphics.Path
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.Canvas
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.spring
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import com.jocmp.mariner.ffi.Article
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min

/** How far past the end a pull has to go to open the next article. */
private val Threshold = 120.dp

/** How much a pull past the end moves the page: it resists, as overscroll does. */
private const val Resistance = 0.5f

/**
 * Pulling past the top or bottom of the article opens the previous or next
 * one. The page stays put; a round indicator grows out of that edge like the
 * droplet of Android's back gesture, and drops free at the threshold.
 * Letting go past it opens the article; short of it, the indicator recedes.
 */
@Composable
fun ArticleSwipe(
  previous: Article?,
  next: Article?,
  onOpen: (String) -> Unit,
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  val threshold = with(LocalDensity.current) { Threshold.toPx() }
  val haptics = LocalHapticFeedback.current
  // Positive pulls down from the top (previous), negative up from the bottom (next).
  var offset by remember { mutableFloatStateOf(0f) }
  val currentPrevious by rememberUpdatedState(previous)
  val currentNext by rememberUpdatedState(next)
  val currentOnOpen by rememberUpdatedState(onOpen)
  val progress = min(1f, abs(offset) / threshold)

  LaunchedEffect(progress >= 1f) {
    if (progress >= 1f) haptics.performHapticFeedback(HapticFeedbackType.GestureThresholdActivate)
  }

  val connection = remember {
    object : NestedScrollConnection {
      /** Scrolling back toward the article undoes the pull first. */
      override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
        if (source != NestedScrollSource.UserInput || offset == 0f) return Offset.Zero
        val towardRest = (offset > 0 && available.y < 0) || (offset < 0 && available.y > 0)
        if (!towardRest) return Offset.Zero
        val before = offset
        offset = if (before > 0) max(0f, before + available.y) else min(0f, before + available.y)
        return Offset(0f, offset - before)
      }

      /** What the article couldn't scroll, at its top or bottom, pulls the page. */
      override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
        if (source != NestedScrollSource.UserInput || available.y == 0f) return Offset.Zero
        val pullingDown = available.y > 0
        if (pullingDown && currentPrevious == null) return Offset.Zero
        if (!pullingDown && currentNext == null) return Offset.Zero
        offset += available.y * Resistance
        return available
      }

      override suspend fun onPreFling(available: Velocity): Velocity {
        if (offset == 0f) return Velocity.Zero
        val target = if (offset > 0) currentPrevious else currentNext
        if (abs(offset) >= threshold && target != null) {
          currentOnOpen(target.id)
          offset = 0f
        } else {
          animate(offset, 0f, animationSpec = spring(stiffness = Spring.StiffnessMediumLow)) { value, _ ->
            offset = value
          }
        }
        return available
      }
    }
  }

  Box(modifier.fillMaxSize().nestedScroll(connection)) {
    content()
    if (offset != 0f) {
      val pullingDown = offset > 0
      Droplet(
        progress = progress,
        pull = abs(offset),
        modifier =
          Modifier.align(if (pullingDown) Alignment.TopCenter else Alignment.BottomCenter)
            // Drawn from the top edge; flipped for the bottom.
            .graphicsLayer { scaleY = if (pullingDown) 1f else -1f },
      )
    }
  }
}

/** The indicator's circle at full size. */
private val Radius = 24.dp

/**
 * A circle that grows out of the edge as the pull grows, joined to it by a
 * neck that thins until, at the threshold, the circle drops free and fills
 * with the primary color: the droplet of Android's back gesture, at the
 * top or bottom.
 */
@Composable
private fun Droplet(progress: Float, pull: Float, modifier: Modifier) {
  val armed = progress >= 1f
  val colors = MaterialTheme.colorScheme
  val fill = if (armed) colors.primary else colors.secondaryContainer
  val iconTint = if (armed) colors.onPrimary else colors.onSecondaryContainer
  val density = LocalDensity.current
  val radius = with(density) { Radius.toPx() } * (0.5f + 0.5f * progress)
  // The circle's center, from the edge: it follows the pull, a little behind.
  val center = min(pull * 0.6f, with(density) { (Threshold * 0.6f).toPx() }) + radius * 0.4f
  val height = with(density) { (center + radius).toDp() + 8.dp }

  Box(modifier.fillMaxWidth().height(height)) {
    Canvas(Modifier.matchParentSize()) {
      val cx = size.width / 2
      drawCircle(fill, radius, Offset(cx, center))
      if (!armed) {
        // The neck: from a base on the edge, curving in to the circle's sides.
        // Wide where it leaves the edge, pinched in the middle like a drop
        // about to fall, and pinching more the further it's pulled.
        val base = radius * (1.8f - 1.0f * progress)
        val waist = radius * (0.55f - 0.4f * progress)
        val neck = Path().apply {
          moveTo(cx - base, 0f)
          cubicTo(cx - waist, center * 0.25f, cx - waist, center - radius, cx - radius * 0.9f, center)
          lineTo(cx + radius * 0.9f, center)
          cubicTo(cx + waist, center - radius, cx + waist, center * 0.25f, cx + base, 0f)
          close()
        }
        drawPath(neck, fill)
      }
    }
    Icon(
      Icons.Default.KeyboardArrowUp,
      contentDescription = null,
      tint = iconTint,
      modifier =
        Modifier.align(Alignment.TopCenter)
          .offset { IntOffset(0, (center - radius * 0.6f).toInt()) }
          .size(with(density) { (radius * 1.2f).toDp() })
          // Flipped with the rest for the bottom, where it points down.
          .graphicsLayer { alpha = progress },
    )
  }
}
