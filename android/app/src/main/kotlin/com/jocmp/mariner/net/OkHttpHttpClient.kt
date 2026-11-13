package com.jocmp.mariner.net

import com.jocmp.mariner.ffi.HttpClient
import com.jocmp.mariner.ffi.HttpException
import com.jocmp.mariner.ffi.HttpHeader
import com.jocmp.mariner.ffi.HttpRequest
import com.jocmp.mariner.ffi.HttpResponse
import java.io.IOException
import java.net.SocketTimeoutException
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.suspendCancellableCoroutine
import okhttp3.Call
import okhttp3.Callback
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response

/**
 * Core's HTTP client: one request, answered as it came. `client` mustn't
 * follow redirects; core follows them, so it can record each one.
 */
class OkHttpHttpClient(private val client: OkHttpClient, private val userAgent: String) : HttpClient {
  override suspend fun send(request: HttpRequest): HttpResponse {
    val builder =
      try {
        Request.Builder().url(request.url)
      } catch (e: IllegalArgumentException) {
        throw HttpException.Request("Not a URL: ${request.url}")
      }
    builder.header("User-Agent", userAgent)
    request.headers.forEach { builder.addHeader(it.name, it.value) }
    val (response, body) =
      try {
        client.newCall(builder.build()).await()
      } catch (e: SocketTimeoutException) {
        throw HttpException.Timeout()
      } catch (e: IOException) {
        throw HttpException.Connect(e.message ?: e.toString())
      }
    return HttpResponse(
      status = response.code.toUShort(),
      headers = response.headers.map { (name, value) -> HttpHeader(name, value) },
      body = body,
    )
  }
}

/** Runs the call, reading its body on OkHttp's thread. */
private suspend fun Call.await(): Pair<Response, ByteArray> = suspendCancellableCoroutine { continuation ->
  enqueue(
    object : Callback {
      override fun onResponse(call: Call, response: Response) {
        try {
          val body = response.use { it.body.bytes() }
          continuation.resume(response to body)
        } catch (e: IOException) {
          continuation.resumeWithException(e)
        }
      }

      override fun onFailure(call: Call, e: IOException) {
        continuation.resumeWithException(e)
      }
    }
  )
  continuation.invokeOnCancellation { cancel() }
}
