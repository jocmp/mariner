package com.jocmp.mariner.refresh

import android.content.Context
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.jocmp.mariner.mariner
import java.util.concurrent.TimeUnit

/** Refreshes the account in the background, about hourly, when online. */
class RefreshWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {
  override suspend fun doWork(): Result =
    try {
      val app = applicationContext.mariner
      app.account.await().refresh(null)
      app.backgroundRefreshes.value += 1
      Result.success()
    } catch (e: Exception) {
      Result.retry()
    }

  companion object {
    fun schedule(context: Context) {
      val request =
        PeriodicWorkRequestBuilder<RefreshWorker>(1, TimeUnit.HOURS)
          .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
          .build()
      WorkManager.getInstance(context)
        .enqueueUniquePeriodicWork("refresh", ExistingPeriodicWorkPolicy.KEEP, request)
    }
  }
}
