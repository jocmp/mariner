package com.jocmp.mariner.ui.reader

import androidx.compose.animation.core.animate
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
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

/** How far past the end a pull goes to open the neighbor. */
private val Threshold = 96.dp

private val Size = 40.dp

/**
 * Pulling past the top or bottom of the article opens the previous or next
 * one. The page stays put: a round arrow slides in from that edge and fills
 * with the primary color once letting go would open the article.
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
  val size = with(LocalDensity.current) { Size.toPx() }
  val haptics = LocalHapticFeedback.current
  // Positive pulls down from the top (previous), negative up from the bottom (next).
  var pull by remember { mutableFloatStateOf(0f) }
  val currentPrevious by rememberUpdatedState(previous)
  val currentNext by rememberUpdatedState(next)
  val currentOnOpen by rememberUpdatedState(onOpen)
  val armed = abs(pull) >= threshold

  LaunchedEffect(armed) {
    if (armed) haptics.performHapticFeedback(HapticFeedbackType.GestureThresholdActivate)
  }

  val connection = remember {
    object : NestedScrollConnection {
      /** Dragging back toward the article takes back the pull first. */
      override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
        if (source != NestedScrollSource.UserInput || pull == 0f) return Offset.Zero
        val before = pull
        pull = if (before > 0) (before + available.y).coerceAtLeast(0f) else (before + available.y).coerceAtMost(0f)
        return Offset(0f, pull - before)
      }

      /** A drag the article can't scroll, at its top or bottom, pulls. */
      override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
        if (source != NestedScrollSource.UserInput || available.y == 0f) return Offset.Zero
        if (available.y > 0 && currentPrevious == null) return Offset.Zero
        if (available.y < 0 && currentNext == null) return Offset.Zero
        pull = (pull + available.y / 2).coerceIn(-threshold * 1.5f, threshold * 1.5f)
        return available
      }

      override suspend fun onPreFling(available: Velocity): Velocity {
        if (pull == 0f) return Velocity.Zero
        val target = if (pull > 0) currentPrevious else currentNext
        if (abs(pull) >= threshold && target != null) {
          pull = 0f
          currentOnOpen(target.id)
        } else {
          animate(pull, 0f) { value, _ -> pull = value }
        }
        return available
      }
    }
  }

  Box(modifier.fillMaxSize().nestedScroll(connection)) {
    content()
    if (pull != 0f) {
      val fromTop = pull > 0
      Surface(
        shape = CircleShape,
        shadowElevation = 3.dp,
        color = if (armed) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceContainerHigh,
        contentColor = if (armed) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
        modifier =
          Modifier.align(if (fromTop) Alignment.TopCenter else Alignment.BottomCenter)
            .padding(vertical = 16.dp)
            .size(Size)
            // Starts hidden past the edge and follows the finger in.
            .graphicsLayer {
              val travel = minOf(abs(pull), threshold) - size
              translationY = if (fromTop) travel else -travel
            },
      ) {
        Icon(
          if (fromTop) Icons.Default.KeyboardArrowUp else Icons.Default.KeyboardArrowDown,
          contentDescription = null,
          modifier = Modifier.padding(8.dp),
        )
      }
    }
  }
}
