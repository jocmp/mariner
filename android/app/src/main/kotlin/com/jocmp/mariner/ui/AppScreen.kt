package com.jocmp.mariner.ui

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.width
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.PermanentDrawerSheet
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.adaptive.ExperimentalMaterial3AdaptiveApi
import androidx.compose.material3.adaptive.currentWindowAdaptiveInfo
import androidx.compose.material3.adaptive.navigation3.ListDetailSceneStrategy
import androidx.compose.material3.adaptive.navigation3.rememberListDetailSceneStrategy
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation3.runtime.NavKey
import androidx.navigation3.runtime.entryProvider
import androidx.navigation3.runtime.rememberNavBackStack
import androidx.navigation3.ui.NavDisplay
import androidx.window.core.layout.WindowSizeClass
import com.jocmp.mariner.AppViewModel
import com.jocmp.mariner.ui.articles.ArticleListScreen
import com.jocmp.mariner.ui.feeds.FeedDrawer
import com.jocmp.mariner.ui.reader.ReaderScreen
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable

@Serializable data object ArticlesKey : NavKey

@Serializable data class ReaderKey(val articleId: String) : NavKey

/**
 * Feeds in a drawer (always open on wide screens), then the articles and the
 * reader: stacked on phones, side by side when there's room.
 */
@OptIn(ExperimentalMaterial3AdaptiveApi::class)
@Composable
fun AppScreen(model: AppViewModel = viewModel()) {
  val wide =
    currentWindowAdaptiveInfo()
      .windowSizeClass
      .isWidthAtLeastBreakpoint(WindowSizeClass.WIDTH_DP_EXPANDED_LOWER_BOUND)
  val drawerState = rememberDrawerState(DrawerValue.Closed)
  val scope = rememberCoroutineScope()
  val backStack = rememberNavBackStack(ArticlesKey)
  val snackbars = remember { SnackbarHostState() }

  LifecycleResumeEffect(Unit) {
    scope.launch { model.reload() }
    onPauseOrDispose {}
  }
  LaunchedEffect(model.message) {
    model.message?.let {
      snackbars.showSnackbar(it)
      model.message = null
    }
  }

  val openArticle: (String) -> Unit = { id ->
    if (backStack.lastOrNull() is ReaderKey) backStack.removeAt(backStack.lastIndex)
    backStack.add(ReaderKey(id))
  }
  val drawer =
    @Composable {
      FeedDrawer(
        model = model,
        onSelect = {
          model.select(it)
          while (backStack.size > 1) backStack.removeAt(backStack.lastIndex)
          scope.launch { drawerState.close() }
        },
      )
    }
  val content =
    @Composable {
      NavDisplay(
        backStack = backStack,
        onBack = { backStack.removeLastOrNull() },
        sceneStrategy = rememberListDetailSceneStrategy<NavKey>(),
        entryProvider =
          entryProvider {
            entry<ArticlesKey>(metadata = ListDetailSceneStrategy.listPane()) {
              ArticleListScreen(
                model = model,
                selectedArticleId = (backStack.lastOrNull() as? ReaderKey)?.articleId,
                showMenu = !wide,
                onMenu = { scope.launch { drawerState.open() } },
                onOpen = openArticle,
                snackbars = snackbars,
              )
            }
            entry<ReaderKey>(metadata = ListDetailSceneStrategy.detailPane()) { key ->
              ReaderScreen(
                model = model,
                articleId = key.articleId,
                showBack = !wide,
                onBack = { backStack.removeLastOrNull() },
              )
            }
          },
      )
    }

  Surface {
    if (wide) {
      Row {
        PermanentDrawerSheet(Modifier.width(300.dp).fillMaxHeight()) { drawer() }
        content()
      }
    } else {
      ModalNavigationDrawer(
        drawerState = drawerState,
        drawerContent = { ModalDrawerSheet { drawer() } },
      ) {
        content()
      }
    }
  }
  SnackbarHost(snackbars)
}
