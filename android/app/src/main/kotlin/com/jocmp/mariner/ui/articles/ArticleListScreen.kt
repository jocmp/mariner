package com.jocmp.mariner.ui.articles

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.DoneAll
import androidx.compose.material.icons.filled.FilterList
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.jocmp.mariner.AppViewModel
import com.jocmp.mariner.SidebarItem
import com.jocmp.mariner.ffi.Article
import com.jocmp.mariner.ffi.ArticleFilter
import com.jocmp.mariner.ffi.Feed
import com.jocmp.mariner.ui.feeds.FeedIcon
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ArticleListScreen(
  model: AppViewModel,
  selectedArticleId: String?,
  onMenu: () -> Unit,
  onOpen: (String) -> Unit,
  snackbars: SnackbarHostState,
) {
  Scaffold(
    topBar = {
      TopAppBar(
        title = { Text(title(model)) },
        navigationIcon = {
          IconButton(onClick = onMenu) { Icon(Icons.Default.Menu, contentDescription = "Feeds") }
        },
        actions = { FilterMenu(model) },
      )
    },
    snackbarHost = { SnackbarHost(snackbars) },
  ) { padding ->
    PullToRefreshBox(
      isRefreshing = model.isRefreshing,
      onRefresh = model::refresh,
      modifier = Modifier.padding(padding).fillMaxSize(),
    ) {
      if (model.isLoaded && model.articles.isEmpty()) {
        EmptyArticles(model.filter)
      }
      LazyColumn(Modifier.fillMaxSize()) {
        items(model.articles, key = { it.id }) { article ->
          ArticleRow(
            article = article,
            feed = model.feed(article.feedId),
            selected = article.id == selectedArticleId,
            onClick = { onOpen(article.id) },
          )
          HorizontalDivider(Modifier.padding(start = 40.dp))
        }
      }
    }
  }
}

private fun title(model: AppViewModel): String =
  when (val selection = model.selection) {
    SidebarItem.All -> "All Articles"
    is SidebarItem.Folder -> selection.name
    is SidebarItem.FeedItem -> model.feed(selection.id)?.title ?: "Feed"
  }

@Composable
private fun FilterMenu(model: AppViewModel) {
  var open by remember { mutableStateOf(false) }
  IconButton(onClick = { open = true }) { Icon(Icons.Default.FilterList, contentDescription = "Filter") }
  DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
    listOf(ArticleFilter.UNREAD to "Unread", ArticleFilter.ALL to "All", ArticleFilter.STARRED to "Starred")
      .forEach { (filter, label) ->
        DropdownMenuItem(
          text = { Text(label) },
          leadingIcon = { if (model.filter == filter) Icon(Icons.Default.Check, contentDescription = null) },
          onClick = {
            open = false
            model.changeFilter(filter)
          },
        )
      }
    HorizontalDivider()
    DropdownMenuItem(
      text = { Text("Mark All as Read") },
      leadingIcon = { Icon(Icons.Default.DoneAll, contentDescription = null) },
      onClick = {
        open = false
        model.markAllRead()
      },
    )
  }
}

@Composable
private fun EmptyArticles(filter: ArticleFilter) {
  Box(Modifier.fillMaxSize().padding(32.dp), contentAlignment = Alignment.Center) {
    Text(
      when (filter) {
        ArticleFilter.UNREAD -> "No unread articles. Pull to refresh."
        ArticleFilter.STARRED -> "No starred articles."
        ArticleFilter.ALL -> "No articles yet. Pull to refresh."
      },
      color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
  }
}

private val dateFormat = DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM)

@Composable
private fun ArticleRow(article: Article, feed: Feed?, selected: Boolean, onClick: () -> Unit) {
  val colors = MaterialTheme.colorScheme
  Row(
    modifier =
      Modifier.fillMaxWidth()
        .background(if (selected) colors.secondaryContainer else colors.surface)
        .clickable(onClick = onClick)
        .padding(horizontal = 16.dp, vertical = 12.dp),
    horizontalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    Box(Modifier.width(16.dp).padding(top = 6.dp), contentAlignment = Alignment.TopCenter) {
      when {
        article.starred -> Icon(Icons.Default.Star, null, Modifier.size(14.dp), tint = colors.tertiary)
        !article.read -> Box(Modifier.size(8.dp).background(colors.primary, CircleShape))
      }
    }
    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
      Text(
        article.title ?: "Untitled",
        style = MaterialTheme.typography.titleMedium,
        fontWeight = if (article.read) FontWeight.Normal else FontWeight.SemiBold,
        color = if (article.read) colors.onSurfaceVariant else colors.onSurface,
        maxLines = 3,
        overflow = TextOverflow.Ellipsis,
      )
      val date = article.publishedAt.atZone(ZoneId.systemDefault()).format(dateFormat)
      Text(
        listOfNotNull(feed?.title, date).joinToString(" · "),
        style = MaterialTheme.typography.bodySmall,
        color = colors.onSurfaceVariant,
      )
    }
    if (feed != null) {
      Spacer(Modifier.width(4.dp))
      FeedIcon(feed, Modifier.padding(top = 2.dp))
    }
  }
}
