package com.jocmp.mariner

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.jocmp.mariner.ui.AppScreen
import com.jocmp.mariner.ui.theme.MarinerTheme

class MainActivity : ComponentActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    enableEdgeToEdge()
    setContent { MarinerTheme { AppScreen() } }
  }
}
