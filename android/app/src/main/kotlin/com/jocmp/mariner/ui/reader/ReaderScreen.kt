package com.jocmp.mariner.ui.reader

import android.content.Intent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Article
import androidx.compose.material.icons.automirrored.outlined.Article
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.StarBorder
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.jocmp.mariner.AppViewModel
import com.jocmp.mariner.ffi.Article
import com.jocmp.mariner.ffi.ArticleContent
import com.jocmp.mariner.ffi.ArticleDocument
import com.jocmp.mariner.ffi.condense
import com.jocmp.mariner.userMessage
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * An article drawn from condense's blocks: the feed's content, or the full
 * text from its page. Choosing the full text sticks for the feed, and a feed
 * that opens in full text shows its own content only when that fails.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReaderScreen(
  model: AppViewModel,
  articleId: String,
  showBack: Boolean,
  onBack: () -> Unit,
  onOpen: (String) -> Unit,
) {
  val scope = rememberCoroutineScope()
  var content by remember(articleId) { mutableStateOf<ArticleContent?>(null) }
  var feedDocument by remember(articleId) { mutableStateOf<ArticleDocument?>(null) }
  var fullTextDocument by remember(articleId) { mutableStateOf<ArticleDocument?>(null) }
  var wantsFullText by remember(articleId) { mutableStateOf(false) }
  var extracting by remember(articleId) { mutableStateOf(false) }
  var fullTextFailure by remember(articleId) { mutableStateOf<String?>(null) }
  // The list's copy, so the header and toolbar show at once.
  val article: Article? = model.articles.find { it.id == articleId } ?: content?.article

  suspend fun loadFeedContent() {
    val loaded = content ?: return
    if (feedDocument != null) return
    feedDocument = condensed(loaded.contentHtml ?: loaded.article.summary, loaded.article.url)
  }

  suspend fun loadFullText() {
    val loaded = content?.article ?: return
    if (fullTextDocument != null || extracting) return
    extracting = true
    fullTextFailure = null
    try {
      val html = model.fullText(loaded)
      if (html == null) fullTextFailure = "No full text found for this article."
      else fullTextDocument = condensed(html, loaded.url)
    } catch (e: Exception) {
      fullTextFailure = "Couldn't get the full text: ${userMessage(e)}"
    }
    extracting = false
    if (fullTextDocument == null) loadFeedContent()
  }

  LaunchedEffect(articleId) {
    article?.feedId?.let { if (model.feed(it)?.stickyFullContent == true) wantsFullText = true }
    val loaded = model.article(articleId) ?: return@LaunchedEffect
    content = loaded
    if (!loaded.article.read) model.setRead(articleId, true)
    if (wantsFullText) loadFullText() else loadFeedContent()
  }

  fun toggleFullText() {
    val feedId = article?.feedId ?: return
    if (wantsFullText && (fullTextDocument != null || extracting)) {
      wantsFullText = false
      model.setStickyFullText(feedId, false)
      scope.launch { loadFeedContent() }
      return
    }
    wantsFullText = true
    model.setStickyFullText(feedId, true)
    scope.launch { loadFullText() }
  }

  val shown = if (wantsFullText) fullTextDocument ?: feedDocument else feedDocument
  // Its neighbors in the list, for pulling past the top or bottom.
  val index = model.articles.indexOfFirst { it.id == articleId }
  val previous = model.articles.getOrNull(index - 1).takeIf { index >= 0 }
  val next = model.articles.getOrNull(index + 1).takeIf { index >= 0 }
  Scaffold(
    topBar = {
      TopAppBar(
        title = {},
        navigationIcon = {
          if (showBack) {
            IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back") }
          }
        },
        actions = { Actions(model, article, wantsFullText, ::toggleFullText) },
      )
    }
  ) { padding ->
    ArticleSwipe(previous, next, onOpen, Modifier.padding(padding)) {
    LazyColumn(
      Modifier.fillMaxSize(),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      val column = Modifier.widthIn(max = 720.dp).fillMaxWidth().padding(horizontal = 20.dp)
      article?.let { item { Header(model, it, column.padding(bottom = 16.dp)) } }
      if (wantsFullText) {
        fullTextFailure?.let { failure ->
          item {
            Text(
              failure,
              style = MaterialTheme.typography.bodyMedium,
              color = MaterialTheme.colorScheme.onSurfaceVariant,
              modifier = column.padding(bottom = 12.dp),
            )
          }
        }
      }
      if (shown == null) {
        item { SlowLoadingIndicator() }
      } else {
        items(shown.blocks) { block -> BlockView(block, column.padding(bottom = 14.dp)) }
      }
    }
    }
  }
}

private val dateFormat = DateTimeFormatter.ofLocalizedDateTime(FormatStyle.LONG, FormatStyle.SHORT)

@Composable
private fun Header(model: AppViewModel, article: Article, modifier: Modifier) {
  Column(modifier, verticalArrangement = Arrangement.spacedBy(6.dp)) {
    model.feed(article.feedId)?.let {
      Text(it.title, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    Text(article.title ?: "Untitled", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
    val date = article.publishedAt.atZone(ZoneId.systemDefault()).format(dateFormat)
    Text(
      listOfNotNull(article.author, date).joinToString(" · "),
      style = MaterialTheme.typography.bodyMedium,
      color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
  }
}

/** The same buttons for every article, so they stay put between articles. */
@Composable
private fun Actions(model: AppViewModel, article: Article?, wantsFullText: Boolean, onFullText: () -> Unit) {
  val context = LocalContext.current
  val starred = article?.starred == true
  IconButton(onClick = { article?.let { model.setStarred(it.id, !starred) } }, enabled = article != null) {
    Icon(if (starred) Icons.Default.Star else Icons.Default.StarBorder, contentDescription = if (starred) "Unstar" else "Star")
  }
  IconButton(onClick = onFullText, enabled = article?.url != null) {
    Icon(
      if (wantsFullText) Icons.AutoMirrored.Filled.Article else Icons.AutoMirrored.Outlined.Article,
      contentDescription = "Full Text",
    )
  }
  IconButton(
    onClick = {
      val url = article?.url ?: return@IconButton
      val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, url)
      context.startActivity(Intent.createChooser(send, null))
    },
    enabled = article?.url != null,
  ) {
    Icon(Icons.Default.Share, contentDescription = "Share")
  }
}

/**
 * A spinner that shows only once loading has taken a while: 3 seconds.
 * Before that the space stays empty, so a quick load doesn't flash one.
 */
@Composable
private fun SlowLoadingIndicator() {
  var slow by remember { mutableStateOf(false) }
  LaunchedEffect(Unit) {
    delay(3_000)
    slow = true
  }
  Box(Modifier.fillMaxWidth().padding(top = 32.dp), contentAlignment = Alignment.Center) {
    if (slow) CircularProgressIndicator()
  }
}

/** Condenses `html` off the main thread. */
private suspend fun condensed(html: String?, base: String?): ArticleDocument? {
  html ?: return null
  return withContext(Dispatchers.Default) { runCatching { condense(html, base ?: "") }.getOrNull() }
}
