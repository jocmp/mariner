package com.jocmp.mariner.ui.reader

import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
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
import androidx.compose.ui.text.style.TextOverflow
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
 * one. As the pull grows, the page shrinks and rounds like Android's
 * predictive back, and a pill names the article it leads to; letting go past
 * the threshold opens it, and short of it springs back.
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

  Box(
    modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceContainerHighest).nestedScroll(connection)
  ) {
    Box(
      Modifier.fillMaxSize()
        .graphicsLayer {
          val scale = 1f - 0.08f * progress
          scaleX = scale
          scaleY = scale
          translationY = offset * 0.4f
          shape = RoundedCornerShape(32.dp * progress)
          clip = progress > 0f
          shadowElevation = 12.dp.toPx() * progress
        }
        .background(MaterialTheme.colorScheme.surface)
    ) {
      content()
    }
    if (offset != 0f) {
      val pullingDown = offset > 0
      val target = if (pullingDown) previous else next
      if (target != null) {
        Pill(
          label = if (pullingDown) "Previous" else "Next",
          title = target.title ?: "Untitled",
          pullingDown = pullingDown,
          modifier =
            Modifier.align(if (pullingDown) Alignment.TopCenter else Alignment.BottomCenter)
              .padding(vertical = 24.dp, horizontal = 32.dp)
              .graphicsLayer { alpha = progress },
        )
      }
    }
  }
}

/** Where letting go leads. */
@Composable
private fun Pill(label: String, title: String, pullingDown: Boolean, modifier: Modifier) {
  Surface(
    modifier = modifier.widthIn(max = 480.dp),
    shape = RoundedCornerShape(50),
    color = MaterialTheme.colorScheme.secondaryContainer,
    shadowElevation = 4.dp,
  ) {
    Row(Modifier.padding(horizontal = 16.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
      Icon(
        if (pullingDown) Icons.Default.KeyboardArrowUp else Icons.Default.KeyboardArrowDown,
        contentDescription = null,
        modifier = Modifier.size(20.dp),
      )
      Text(
        "$label: $title",
        style = MaterialTheme.typography.labelLarge,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier = Modifier.padding(start = 8.dp),
      )
    }
  }
}
