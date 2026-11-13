package com.jocmp.mariner.ui.dialogs

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.unit.dp
import com.jocmp.mariner.AppViewModel

@Composable
fun RenameDialog(title: String, name: String, onDismiss: () -> Unit, onRename: (String) -> Unit) {
  var text by remember { mutableStateOf(name) }
  val focus = remember { FocusRequester() }
  AlertDialog(
    onDismissRequest = onDismiss,
    title = { Text(title) },
    text = {
      OutlinedTextField(text, { text = it }, singleLine = true, modifier = Modifier.focusRequester(focus))
    },
    confirmButton = {
      TextButton(onClick = { onRename(text.trim()) }, enabled = text.isNotBlank()) { Text("Rename") }
    },
    dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
  )
  LaunchedEffect(Unit) { focus.requestFocus() }
}

@Composable
fun ConfirmDialog(title: String, text: String, confirm: String, onDismiss: () -> Unit, onConfirm: () -> Unit) {
  AlertDialog(
    onDismissRequest = onDismiss,
    title = { Text(title) },
    text = { Text(text) },
    confirmButton = { TextButton(onClick = onConfirm) { Text(confirm) } },
    dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
  )
}

/**
 * Folders to check, any number of them, and a new one to name: for a new
 * feed (Add to Folder…) and for moving one (Move to…).
 */
@Composable
fun FolderPickerDialog(
  model: AppViewModel,
  title: String,
  confirm: String,
  initiallyChecked: List<String> = emptyList(),
  onDismiss: () -> Unit,
  onConfirm: (List<String>) -> Unit,
) {
  val folders = remember { mutableStateListOf<String>() }
  val checked = remember { mutableStateListOf<String>().apply { addAll(initiallyChecked) } }
  var newFolder by remember { mutableStateOf("") }
  LaunchedEffect(Unit) {
    val names = model.folderNames()
    folders.addAll(names + initiallyChecked.filter { it !in names })
  }
  fun addNewFolder() {
    val name = newFolder.trim()
    if (name.isEmpty()) return
    if (name !in folders) folders.add(name)
    if (name !in checked) checked.add(name)
    newFolder = ""
  }
  AlertDialog(
    onDismissRequest = onDismiss,
    title = { Text(title) },
    text = {
      Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        LazyColumn(Modifier.heightIn(max = 280.dp)) {
          items(folders) { folder ->
            Row(
              Modifier.fillMaxWidth().clickable {
                if (folder in checked) checked.remove(folder) else checked.add(folder)
              },
              verticalAlignment = Alignment.CenterVertically,
            ) {
              Checkbox(checked = folder in checked, onCheckedChange = null)
              Text(folder)
            }
          }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
          OutlinedTextField(
            newFolder,
            { newFolder = it },
            label = { Text("New folder") },
            singleLine = true,
            modifier = Modifier.weight(1f),
          )
          TextButton(onClick = ::addNewFolder, enabled = newFolder.isNotBlank()) { Text("Add") }
        }
      }
    },
    confirmButton = {
      TextButton(
        onClick = {
          addNewFolder()
          onConfirm(folders.filter { it in checked })
        }
      ) {
        Text(confirm)
      }
    },
    dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
  )
}
