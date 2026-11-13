package com.jocmp.mariner

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.jocmp.mariner.ffi.Account
import com.jocmp.mariner.ffi.Article
import com.jocmp.mariner.ffi.ArticleContent
import com.jocmp.mariner.ffi.ArticleFilter
import com.jocmp.mariner.ffi.ArticleScope
import com.jocmp.mariner.ffi.Feed
import com.jocmp.mariner.ffi.FeedChoice
import com.jocmp.mariner.ffi.OpmlImport
import com.jocmp.mariner.ffi.Tag
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch

/** What the drawer selects. */
sealed interface SidebarItem {
  data object All : SidebarItem

  data class Folder(val name: String) : SidebarItem

  data class FeedItem(val id: String) : SidebarItem

  val scope: ArticleScope
    get() =
      when (this) {
        All -> ArticleScope.All
        is Folder -> ArticleScope.Tag(name)
        is FeedItem -> ArticleScope.Feed(id)
      }
}

/** The account and what the screens show from it. */
class AppViewModel(application: Application) : AndroidViewModel(application) {
  var feeds by mutableStateOf<List<Feed>>(emptyList())
    private set

  var folders by mutableStateOf<List<Tag>>(emptyList())
    private set

  var unreadCounts by mutableStateOf<Map<String, Long>>(emptyMap())
    private set

  var articles by mutableStateOf<List<Article>>(emptyList())
    private set

  var selection by mutableStateOf<SidebarItem>(SidebarItem.All)
    private set

  var filter by mutableStateOf(ArticleFilter.UNREAD)
    private set

  var isRefreshing by mutableStateOf(false)
    private set

  var isLoaded by mutableStateOf(false)
    private set

  /** An error to show once, then clear. */
  var message by mutableStateOf<String?>(null)

  /**
   * The article the reader shows, if any. A refresh keeps it listed even
   * when the filter would leave it out.
   */
  var openArticleId: String? = null

  /** Full texts extracted this session, by article id. */
  private val fullTexts = mutableMapOf<String, String?>()

  init {
    viewModelScope.launch { reload() }
    // A background refresh counts as a refresh.
    viewModelScope.launch {
      getApplication<MarinerApplication>().backgroundRefreshes.drop(1).collect { reload() }
    }
  }

  /**
   * Reloads feeds, folders, counts and the article list from the database.
   * In between, the list only changes in place. The open article stays
   * listed.
   */
  suspend fun reload() {
    reloadFeeds()
    loadArticles(keep = openArticleId)
    isLoaded = true
  }

  /** Reloads feeds, folders and counts, leaving the article list alone. */
  suspend fun reloadFeeds() {
    attempt {
      val account = account()
      feeds = account.feeds()
      val tagged = feeds.flatMap { it.tagNames }.toSet()
      folders = account.tags().filter { it.name in tagged }
      unreadCounts = account.unreadCounts().mapValues { it.value.toLong() }
    }
  }

  /** Lists `item`'s articles. The caller closes the open article. */
  fun select(item: SidebarItem) {
    selection = item
    viewModelScope.launch { loadArticles(keep = null) }
  }

  /** Lists the articles `filter` keeps. The caller closes the open article. */
  fun changeFilter(filter: ArticleFilter) {
    this.filter = filter
    viewModelScope.launch { loadArticles(keep = null) }
  }

  fun refresh() {
    if (isRefreshing) return
    viewModelScope.launch {
      isRefreshing = true
      attempt { account().refresh(null) }
      reload()
      isRefreshing = false
    }
  }

  fun feed(id: String): Feed? = feeds.find { it.id == id }

  fun feedsIn(folder: String): List<Feed> = feeds.filter { folder in it.tagNames }

  val feedsWithoutFolder: List<Feed>
    get() = feeds.filter { it.tagNames.isEmpty() }

  fun unreadCount(item: SidebarItem): Long =
    when (item) {
      SidebarItem.All -> unreadCounts.values.sum()
      is SidebarItem.Folder -> feedsIn(item.name).sumOf { unreadCounts[it.id] ?: 0 }
      is SidebarItem.FeedItem -> unreadCounts[item.id] ?: 0
    }

  fun setExpanded(folder: String, expanded: Boolean) {
    folders = folders.map { if (it.name == folder) it.copy(expanded = expanded) else it }
    viewModelScope.launch { attempt { account().setTagExpanded(folder, expanded) } }
  }

  suspend fun article(id: String): ArticleContent? = attempt { account().article(id) }

  /** Marks an article read or unread, updating the list in place. */
  fun setRead(id: String, read: Boolean) {
    viewModelScope.launch {
      attempt {
        val account = account()
        if (read) account.markRead(listOf(id)) else account.markUnread(listOf(id))
        updateListed(id) { it.copy(read = read) }
        unreadCounts = account.unreadCounts().mapValues { it.value.toLong() }
      }
    }
  }

  fun setStarred(id: String, starred: Boolean) {
    viewModelScope.launch {
      attempt {
        if (starred) account().addStar(listOf(id)) else account().removeStar(listOf(id))
        updateListed(id) { it.copy(starred = starred) }
      }
    }
  }

  fun markAllRead() {
    viewModelScope.launch {
      attempt { account().markAllRead(selection.scope) }
      reload()
    }
  }

  /** The article's full text, extracted from its page once per session. */
  suspend fun fullText(article: Article): String? {
    if (fullTexts.containsKey(article.id)) return fullTexts[article.id]
    val url = article.url ?: return null
    val html = account().fullContent(url)
    fullTexts[article.id] = html
    return html
  }

  /** Whether the feed's articles open in full text. */
  fun setStickyFullText(feedId: String, enabled: Boolean) {
    val feed = feed(feedId) ?: return
    if (feed.stickyFullContent == enabled) return
    feeds = feeds.map { if (it.id == feedId) it.copy(stickyFullContent = enabled) else it }
    viewModelScope.launch { attempt { account().setStickyFullContent(feedId, enabled) } }
  }

  suspend fun findFeeds(url: String): List<FeedChoice> = account().findFeeds(url)

  /** Subscribes, then, as its own change, adds the feed to `folders`. */
  suspend fun subscribe(url: String, folders: List<String>) {
    val account = account()
    var feed = account.subscribe(url, null)
    if (folders.isNotEmpty()) {
      feed = account.updateFeed(feed, feed.title, (feed.tagNames + folders).distinct())
      folders.forEach { account.setTagExpanded(it, true) }
    }
    reload()
    select(SidebarItem.FeedItem(feed.id))
  }

  suspend fun folderNames(): List<String> = attempt { account().tags().map { it.name } } ?: emptyList()

  fun renameFeed(id: String, title: String) {
    val feed = feed(id) ?: return
    viewModelScope.launch {
      attempt { account().updateFeed(feed, title, feed.tagNames) }
      reload()
    }
  }

  /** Puts a feed in exactly `folders`. Folders it leaves empty go. */
  fun moveFeed(id: String, folders: List<String>) {
    val feed = feed(id) ?: return
    viewModelScope.launch {
      attempt {
        account().updateFeed(feed, feed.title, folders)
        folders.forEach { account().setTagExpanded(it, true) }
      }
      reload()
    }
  }

  fun removeFeed(id: String) {
    val feed = feed(id) ?: return
    viewModelScope.launch {
      attempt { account().removeFeed(feed) }
      if (selection == SidebarItem.FeedItem(id)) selection = SidebarItem.All
      reload()
    }
  }

  fun renameFolder(name: String, newName: String) {
    viewModelScope.launch {
      attempt { account().updateTag(name, newName) }
      if (selection == SidebarItem.Folder(name)) selection = SidebarItem.Folder(newName)
      reload()
    }
  }

  fun removeFolder(name: String) {
    viewModelScope.launch {
      attempt { account().removeTag(name) }
      if (selection == SidebarItem.Folder(name)) selection = SidebarItem.All
      reload()
    }
  }

  /** Subscribes to an OPML file's feeds, in its folders, then refreshes. */
  suspend fun importOpml(document: ByteArray): OpmlImport {
    val imported = account().importOpml(document)
    reload()
    refresh()
    return imported
  }

  suspend fun exportOpml(): String = account().exportOpml()

  private suspend fun loadArticles(keep: String?) {
    attempt { articles = account().articles(filter, selection.scope, keep) }
  }

  private suspend fun account(): Account = getApplication<MarinerApplication>().account.await()

  /** Runs `block`, showing its error, if any, as the message. */
  private suspend fun <T> attempt(block: suspend () -> T): T? =
    try {
      block()
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      message = userMessage(e)
      null
    }

  private fun updateListed(id: String, change: (Article) -> Article) {
    articles = articles.map { if (it.id == id) change(it) else it }
  }
}
