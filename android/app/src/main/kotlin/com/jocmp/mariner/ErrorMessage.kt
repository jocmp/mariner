package com.jocmp.mariner

import com.jocmp.mariner.ffi.MarinerException

/** What to tell the user about an error: the reason core gives. */
fun userMessage(error: Throwable): String =
  when (error) {
    is MarinerException.Database -> error.reason
    is MarinerException.Network -> error.reason
    is MarinerException.Extract -> error.reason
    is MarinerException.Opml -> error.reason
    is MarinerException.Condense -> error.reason
    is MarinerException.NoFeed -> "There's no feed at ${error.url}."
    is MarinerException.Status -> "${error.url} answered with HTTP ${error.status}."
    else -> error.message ?: error.toString()
  }
