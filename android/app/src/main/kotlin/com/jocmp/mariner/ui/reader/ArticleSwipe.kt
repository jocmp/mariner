package com.jocmp.mariner.ui.reader

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.jocmp.mariner.ffi.Article
import com.jocmp.swiperefresh.SwipeRefresh
import com.jocmp.swiperefresh.SwipeRefreshIndicator
import com.jocmp.swiperefresh.rememberSwipeRefreshState

/**
 * Pulling past the top or bottom of the article opens the previous or next
 * one, with swipe to refresh's indicator showing an arrow.
 */
@Composable
fun ArticleSwipe(
  previous: Article?,
  next: Article?,
  onOpen: (String) -> Unit,
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  SwipeRefresh(
    state = rememberSwipeRefreshState(isRefreshing = false),
    onRefresh = { previous?.let { onOpen(it.id) } },
    swipeEnabled = previous != null,
    indicator = { state, trigger -> SwipeRefreshIndicator(state, trigger, icon = Icons.Default.KeyboardArrowUp) },
    modifier = modifier,
  ) {
    SwipeRefresh(
      state = rememberSwipeRefreshState(isRefreshing = false),
      onRefresh = { next?.let { onOpen(it.id) } },
      swipeEnabled = next != null,
      indicatorAlignment = Alignment.BottomCenter,
      indicator = { state, trigger ->
        SwipeRefreshIndicator(state, trigger, clockwise = false, icon = Icons.Default.KeyboardArrowDown)
      },
      content = content,
    )
  }
}
