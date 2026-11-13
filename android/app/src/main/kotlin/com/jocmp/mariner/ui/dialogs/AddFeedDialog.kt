package com.jocmp.mariner.ui.dialogs

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.jocmp.mariner.AppViewModel
import com.jocmp.mariner.ffi.FeedChoice
import com.jocmp.mariner.userMessage
import kotlinx.coroutines.launch

/**
 * A URL box, then the feeds it leads to (always, even one) with
 * "Add to Folder…" and "Subscribe" beneath. Subscribing here adds the feed
 * to no folder.
 */
@Composable
fun AddFeedDialog(model: AppViewModel, onDismiss: () -> Unit) {
  val scope = rememberCoroutineScope()
  var url by remember { mutableStateOf("") }
  var choices by remember { mutableStateOf<List<FeedChoice>>(emptyList()) }
  var selected by remember { mutableStateOf<String?>(null) }
  var working by remember { mutableStateOf(false) }
  var error by remember { mutableStateOf<String?>(null) }
  var choosingFolders by remember { mutableStateOf(false) }
  val focus = remember { FocusRequester() }

  fun find() {
    val typed = url.trim()
    if (typed.isEmpty() || working) return
    working = true
    error = null
    scope.launch {
      try {
        val found = model.findFeeds(typed)
        if (found.isEmpty()) error = "No feed found there." else {
          choices = found
          selected = found.first().url
        }
      } catch (e: Exception) {
        error = userMessage(e)
      }
      working = false
    }
  }

  fun subscribe(folders: List<String>) {
    val choice = selected ?: return
    working = true
    error = null
    scope.launch {
      try {
        model.subscribe(choice, folders)
        onDismiss()
      } catch (e: Exception) {
        error = userMessage(e)
        working = false
      }
    }
  }

  AlertDialog(
    onDismissRequest = onDismiss,
    title = { Text("Add Feed") },
    text = {
      Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        if (choices.isEmpty()) {
          OutlinedTextField(
            url,
            { url = it },
            label = { Text("Site or feed URL") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { find() }),
            modifier = Modifier.fillMaxWidth().focusRequester(focus),
          )
        } else {
          LazyColumn(Modifier.heightIn(max = 280.dp)) {
            items(choices, key = { it.url }) { choice ->
              Row(
                Modifier.fillMaxWidth().clickable { selected = choice.url },
                verticalAlignment = Alignment.CenterVertically,
              ) {
                RadioButton(selected = selected == choice.url, onClick = null)
                Column {
                  Text(choice.title ?: "Untitled")
                  Text(
                    choice.url,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                  )
                }
              }
            }
          }
          OutlinedButton(
            onClick = { choosingFolders = true },
            enabled = !working,
            modifier = Modifier.fillMaxWidth(),
          ) {
            Text("Add to Folder…")
          }
          Button(onClick = { subscribe(emptyList()) }, enabled = !working, modifier = Modifier.fillMaxWidth()) {
            Text("Subscribe")
          }
        }
        error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
      }
    },
    confirmButton = {
      if (choices.isEmpty()) {
        if (working) CircularProgressIndicator()
        else TextButton(onClick = ::find, enabled = url.isNotBlank()) { Text("Find") }
      }
    },
    dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
  )
  if (choosingFolders) {
    FolderPickerDialog(
      model = model,
      title = "Add to Folder",
      confirm = "Subscribe",
      onDismiss = { choosingFolders = false },
      onConfirm = { folders ->
        choosingFolders = false
        subscribe(folders)
      },
    )
  }
  LaunchedEffect(Unit) { focus.requestFocus() }
}
