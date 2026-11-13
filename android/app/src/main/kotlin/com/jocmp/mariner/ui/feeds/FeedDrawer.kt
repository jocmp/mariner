package com.jocmp.mariner.ui.feeds

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ExpandLess
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material.icons.filled.FileDownload
import androidx.compose.material.icons.filled.FileUpload
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.Inbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.jocmp.mariner.AppViewModel
import com.jocmp.mariner.SidebarItem
import com.jocmp.mariner.ffi.Feed
import com.jocmp.mariner.ui.dialogs.AddFeedDialog
import com.jocmp.mariner.ui.dialogs.ConfirmDialog
import com.jocmp.mariner.ui.dialogs.FolderPickerDialog
import com.jocmp.mariner.ui.dialogs.RenameDialog
import com.jocmp.mariner.userMessage
import kotlinx.coroutines.launch

/** What a context menu acts on. */
private sealed interface Target {
  data class FeedTarget(val feed: Feed) : Target

  data class FolderTarget(val name: String) : Target

  val label: String
    get() =
      when (this) {
        is FeedTarget -> feed.title
        is FolderTarget -> name
      }
}

/**
 * Every article, then folders with their feeds, then feeds in no folder.
 * Feeds and folders are renamed, moved and removed from a long press.
 */
@Composable
fun FeedDrawer(model: AppViewModel, onSelect: (SidebarItem) -> Unit) {
  var adding by remember { mutableStateOf(false) }
  var renaming by remember { mutableStateOf<Target?>(null) }
  var moving by remember { mutableStateOf<Feed?>(null) }
  var removing by remember { mutableStateOf<Target?>(null) }

  LazyColumn(Modifier.padding(horizontal = 12.dp)) {
    item { Header(model, onAdd = { adding = true }) }
    item {
      SidebarRow(SidebarItem.All, "All Articles", model, onSelect, icon = { Icon(Icons.Outlined.Inbox, null) })
    }
    model.folders.forEach { folder ->
      item(key = "folder:${folder.name}") {
        SidebarRow(
          SidebarItem.Folder(folder.name),
          folder.name,
          model,
          onSelect,
          icon = { Icon(Icons.Outlined.Folder, null) },
          trailing = {
            IconButton(onClick = { model.setExpanded(folder.name, !folder.expanded) }) {
              Icon(
                if (folder.expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                contentDescription = if (folder.expanded) "Close" else "Open",
              )
            }
          },
          menu = listOf(
            "Rename…" to { renaming = Target.FolderTarget(folder.name) },
            "Remove" to { removing = Target.FolderTarget(folder.name) },
          ),
        )
      }
      if (folder.expanded) {
        items(model.feedsIn(folder.name), key = { "${folder.name}/${it.id}" }) { feed ->
          FeedRow(feed, model, onSelect, indent = true, { renaming = it }, { moving = it }, { removing = it })
        }
      }
    }
    items(model.feedsWithoutFolder, key = { it.id }) { feed ->
      FeedRow(feed, model, onSelect, indent = false, { renaming = it }, { moving = it }, { removing = it })
    }
  }

  if (adding) AddFeedDialog(model, onDismiss = { adding = false })
  renaming?.let { target ->
    RenameDialog(
      title = if (target is Target.FolderTarget) "Rename Folder" else "Rename Feed",
      name = target.label,
      onDismiss = { renaming = null },
      onRename = { name ->
        renaming = null
        when (target) {
          is Target.FeedTarget -> model.renameFeed(target.feed.id, name)
          is Target.FolderTarget -> model.renameFolder(target.name, name)
        }
      },
    )
  }
  moving?.let { feed ->
    FolderPickerDialog(
      model = model,
      title = "Move to",
      confirm = "Move",
      initiallyChecked = feed.tagNames,
      onDismiss = { moving = null },
      onConfirm = { folders ->
        moving = null
        model.moveFeed(feed.id, folders)
      },
    )
  }
  removing?.let { target ->
    ConfirmDialog(
      title = "Remove ${target.label}?",
      text =
        when (target) {
          is Target.FeedTarget -> "Its articles go too, starred ones included."
          is Target.FolderTarget -> "Its feeds stay."
        },
      confirm = "Remove",
      onDismiss = { removing = null },
      onConfirm = {
        removing = null
        when (target) {
          is Target.FeedTarget -> model.removeFeed(target.feed.id)
          is Target.FolderTarget -> model.removeFolder(target.name)
        }
      },
    )
  }
}

/** The app's name, adding a feed, and OPML import and export. */
@Composable
private fun Header(model: AppViewModel, onAdd: () -> Unit) {
  val context = LocalContext.current
  val scope = rememberCoroutineScope()
  var menu by remember { mutableStateOf(false) }
  val importer =
    rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
      uri ?: return@rememberLauncherForActivityResult
      scope.launch {
        try {
          val bytes = context.contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return@launch
          val imported = model.importOpml(bytes)
          model.message =
            if (imported.existing == 0u) "Subscribed to ${imported.added} feeds. Refreshing…"
            else "Subscribed to ${imported.added} feeds; ${imported.existing} were already here."
        } catch (e: Exception) {
          model.message = userMessage(e)
        }
      }
    }
  val exporter =
    rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("text/x-opml")) { uri ->
      uri ?: return@rememberLauncherForActivityResult
      scope.launch {
        try {
          val opml = model.exportOpml()
          context.contentResolver.openOutputStream(uri)?.use { it.write(opml.toByteArray()) }
        } catch (e: Exception) {
          model.message = userMessage(e)
        }
      }
    }

  Row(
    Modifier.fillMaxWidth().padding(start = 16.dp, top = 12.dp, bottom = 8.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Text("Mariner", style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
    IconButton(onClick = onAdd) { Icon(Icons.Default.Add, contentDescription = "Add Feed") }
    Box {
      IconButton(onClick = { menu = true }) { Icon(Icons.Default.MoreVert, contentDescription = "More") }
      DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
        // Only local accounts import; it's the only kind so far.
        DropdownMenuItem(
          text = { Text("Import OPML…") },
          leadingIcon = { Icon(Icons.Default.FileDownload, null) },
          onClick = {
            menu = false
            importer.launch(arrayOf("*/*"))
          },
        )
        DropdownMenuItem(
          text = { Text("Export OPML…") },
          leadingIcon = { Icon(Icons.Default.FileUpload, null) },
          onClick = {
            menu = false
            exporter.launch("Mariner.opml")
          },
        )
      }
    }
  }
}

@Composable
private fun FeedRow(
  feed: Feed,
  model: AppViewModel,
  onSelect: (SidebarItem) -> Unit,
  indent: Boolean,
  onRename: (Target) -> Unit,
  onMove: (Feed) -> Unit,
  onRemove: (Target) -> Unit,
) {
  SidebarRow(
    SidebarItem.FeedItem(feed.id),
    feed.title,
    model,
    onSelect,
    indent = indent,
    icon = { FeedIcon(feed) },
    menu = listOf(
      "Rename…" to { onRename(Target.FeedTarget(feed)) },
      "Move to…" to { onMove(feed) },
      "Remove" to { onRemove(Target.FeedTarget(feed)) },
    ),
  )
}

/** A row that selects on tap and offers `menu` on a long press. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun SidebarRow(
  item: SidebarItem,
  title: String,
  model: AppViewModel,
  onSelect: (SidebarItem) -> Unit,
  indent: Boolean = false,
  icon: @Composable () -> Unit,
  trailing: @Composable () -> Unit = {},
  menu: List<Pair<String, () -> Unit>> = emptyList(),
) {
  var showMenu by remember { mutableStateOf(false) }
  val selected = model.selection == item
  val colors = MaterialTheme.colorScheme
  Box {
    Row(
      Modifier.fillMaxWidth()
        .padding(start = if (indent) 20.dp else 0.dp)
        .clip(RoundedCornerShape(28.dp))
        .background(if (selected) colors.secondaryContainer else colors.surface.copy(alpha = 0f))
        .combinedClickable(
          onClick = { onSelect(item) },
          onLongClick = { if (menu.isNotEmpty()) showMenu = true },
        )
        .heightIn(min = 52.dp)
        .padding(start = 16.dp, end = 8.dp),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      icon()
      Text(
        title,
        modifier = Modifier.weight(1f),
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
      )
      val count = model.unreadCount(item)
      if (count > 0) {
        Text(count.toString(), style = MaterialTheme.typography.labelLarge, color = colors.onSurfaceVariant)
      }
      trailing()
      Spacer(Modifier.size(4.dp))
    }
    DropdownMenu(expanded = showMenu, onDismissRequest = { showMenu = false }) {
      menu.forEach { (label, action) ->
        DropdownMenuItem(
          text = { Text(label) },
          onClick = {
            showMenu = false
            action()
          },
        )
      }
    }
  }
}

