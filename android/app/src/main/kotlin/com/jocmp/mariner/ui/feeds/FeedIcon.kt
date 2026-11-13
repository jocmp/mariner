package com.jocmp.mariner.ui.feeds

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.SubcomposeAsyncImage
import com.jocmp.mariner.ffi.Feed

/** A feed's icon, or its first letter while there's none to show. */
@Composable
fun FeedIcon(feed: Feed, modifier: Modifier = Modifier, size: Dp = 20.dp) {
  val shape = RoundedCornerShape(size * 0.22f)
  SubcomposeAsyncImage(
    model = feed.faviconUrl,
    contentDescription = null,
    modifier = modifier.size(size).clip(shape),
    loading = { Letter(feed, size) },
    error = { Letter(feed, size) },
  )
}

@Composable
private fun Letter(feed: Feed, size: Dp) {
  Box(
    Modifier.size(size).background(MaterialTheme.colorScheme.surfaceVariant),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      feed.title.take(1).uppercase(),
      fontSize = (size.value * 0.6f).sp,
      fontWeight = FontWeight.SemiBold,
      color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
  }
}
