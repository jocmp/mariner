package com.jocmp.mariner

import android.app.Application
import android.content.Context
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader
import coil3.disk.DiskCache
import coil3.gif.AnimatedImageDecoder
import coil3.network.okhttp.OkHttpNetworkFetcherFactory
import com.jocmp.mariner.ffi.Account
import com.jocmp.mariner.net.OkHttpHttpClient
import com.jocmp.mariner.refresh.RefreshWorker
import java.io.File
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import okhttp3.OkHttpClient
import okio.Path.Companion.toOkioPath

/** The app's account and HTTP, made once for the process. */
class MarinerApplication : Application(), SingletonImageLoader.Factory {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

  private val http: OkHttpClient by lazy {
    OkHttpClient.Builder()
      .connectTimeout(30, TimeUnit.SECONDS)
      .readTimeout(30, TimeUnit.SECONDS)
      .build()
  }

  /**
   * The local account, stored in the app's private files. Core follows
   * redirects itself, recording each one, so its client doesn't.
   */
  val account: Deferred<Account> by lazy {
    scope.async {
      val client = http.newBuilder().followRedirects(false).followSslRedirects(false).build()
      val path = File(filesDir, "accounts/local.db").path
      Account.local(path, OkHttpHttpClient(client, userAgent()))
    }
  }

  override fun onCreate() {
    super.onCreate()
    RefreshWorker.schedule(this)
  }

  /** Article images and feed icons: a disk cache, and GIFs that play. */
  override fun newImageLoader(context: PlatformContext): ImageLoader =
    ImageLoader.Builder(context)
      .components {
        add(OkHttpNetworkFetcherFactory(callFactory = { http }))
        add(AnimatedImageDecoder.Factory())
      }
      .diskCache {
        DiskCache.Builder()
          .directory(context.cacheDir.resolve("images").toOkioPath())
          .maxSizeBytes(256L * 1024 * 1024)
          .build()
      }
      .build()

  private fun userAgent(): String {
    val version = packageManager.getPackageInfo(packageName, 0).versionName
    return "Mariner/$version (+https://jocmp.com)"
  }
}

val Context.mariner: MarinerApplication
  get() = applicationContext as MarinerApplication
