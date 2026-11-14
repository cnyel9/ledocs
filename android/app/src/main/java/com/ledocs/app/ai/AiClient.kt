package com.ledocs.app.ai

import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

/** Kegagalan panggilan provider, sudah diterjemahkan ke kelas yang bisa ditampilkan ke pengguna. */
class AiException(val kind: Kind, message: String, val retryAfterSec: Long? = null) : Exception(message) {
    enum class Kind { AUTH, QUOTA, NETWORK, SERVER, BAD_REQUEST }
}

/**
 * Satu satunya tempat jaringan untuk Asisten AI: POST {baseUrl}/chat/completions kompatibel
 * OpenAI, streaming SSE. Pesan asisten disimpan utuh (content + reasoning_content) karena
 * DeepSeek dan GLM mewajibkan reasoning_content dikirim balik selama percakapan berjalan.
 */
class AiClient {
    @Volatile private var connection: HttpURLConnection? = null

    fun disconnect() {
        runCatching { connection?.disconnect() }
        connection = null
    }

    /**
     * Mengirim [messages] dan mengalirkan teks jawaban ke [onDelta]. Mengembalikan pesan asisten
     * lengkap sebagai JSONObject. Retry sampai 3 kali hanya sebelum streaming mulai (429 dan 5xx,
     * menghormati Retry-After); sekali token mengalir, kegagalan dilaporkan apa adanya.
     */
    suspend fun chatStream(
        provider: AiProvider,
        apiKey: String?,
        messages: JSONArray,
        onDelta: (String) -> Unit,
    ): JSONObject = withContext(Dispatchers.IO) {
        val body = JSONObject().apply {
            put("model", provider.model)
            put("messages", messages)
            put("stream", true)
            put("temperature", 0.7)
        }.toString()
        var attempt = 0
        var lastError: AiException? = null
        while (attempt < 3) {
            attempt++
            try {
                return@withContext streamOnce(provider, apiKey, body, onDelta)
            } catch (e: CancellationException) {
                throw e
            } catch (e: AiException) {
                lastError = e
                val retryable = e.kind == AiException.Kind.QUOTA || e.kind == AiException.Kind.SERVER
                if (!retryable || attempt >= 3) throw e
                val waitMs = e.retryAfterSec?.let { minOf(it * 1000, 10_000L) } ?: minOf(1000L shl (attempt - 1), 4000L)
                delay(waitMs)
            }
        }
        throw lastError ?: AiException(AiException.Kind.NETWORK, "network")
    }

    private suspend fun streamOnce(
        provider: AiProvider,
        apiKey: String?,
        body: String,
        onDelta: (String) -> Unit,
    ): JSONObject {
        val url = URL(provider.baseUrl.trimEnd('/') + "/chat/completions")
        val conn = url.openConnection() as HttpURLConnection
        connection = conn
        try {
            conn.requestMethod = "POST"
            conn.instanceFollowRedirects = false
            conn.connectTimeout = 20_000
            conn.readTimeout = 180_000
            conn.doOutput = true
            conn.setRequestProperty("Content-Type", "application/json")
            conn.setRequestProperty("Accept", "text/event-stream")
            if (!apiKey.isNullOrBlank()) conn.setRequestProperty("Authorization", "Bearer $apiKey")
            conn.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
            val code = conn.responseCode
            if (code !in 200..299) {
                val errBody = runCatching { conn.errorStream?.bufferedReader()?.readText() }.getOrNull().orEmpty()
                val msg = runCatching { JSONObject(errBody).optJSONObject("error")?.optString("message") }.getOrNull()?.takeIf { it.isNotBlank() } ?: "HTTP $code"
                val retryAfter = conn.getHeaderField("Retry-After")?.toLongOrNull()
                throw AiException(
                    when {
                        code == 401 || code == 403 -> AiException.Kind.AUTH
                        code == 429 -> AiException.Kind.QUOTA
                        code >= 500 -> AiException.Kind.SERVER
                        else -> AiException.Kind.BAD_REQUEST
                    },
                    msg,
                    retryAfter,
                )
            }
            val content = StringBuilder()
            val reasoning = StringBuilder()
            val reader = BufferedReader(InputStreamReader(conn.inputStream, Charsets.UTF_8))
            while (true) {
                currentCoroutineContext().ensureActive()
                val line = reader.readLine() ?: break
                if (!line.startsWith("data:")) continue
                val data = line.removePrefix("data:").trim()
                if (data == "[DONE]") break
                val chunk = runCatching { JSONObject(data) }.getOrNull() ?: continue
                val delta = chunk.optJSONArray("choices")?.optJSONObject(0)?.optJSONObject("delta") ?: continue
                val piece = delta.optString("content")
                if (piece.isNotEmpty()) {
                    content.append(piece)
                    onDelta(piece)
                }
                val rPiece = delta.optString("reasoning_content")
                if (rPiece.isNotEmpty()) reasoning.append(rPiece)
            }
            return JSONObject().apply {
                put("role", "assistant")
                put("content", content.toString())
                if (reasoning.isNotEmpty()) put("reasoning_content", reasoning.toString())
            }
        } catch (e: AiException) {
            throw e
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            throw AiException(AiException.Kind.NETWORK, e.message ?: "network")
        } finally {
            if (connection === conn) connection = null
            runCatching { conn.disconnect() }
        }
    }

    companion object {
        /**
         * Validasi base URL provider: wajib https, kecuali host loopback persis (localhost,
         * 127.0.0.1, ::1) yang boleh http untuk server lokal pengguna sendiri. Tanpa userinfo.
         * Mengembalikan URL ternormalisasi (tanpa garis miring akhir) atau null bila tidak sah.
         */
        fun validateBaseUrl(raw: String): String? {
            val url = runCatching { URL(raw.trim()) }.getOrNull() ?: return null
            if (!url.userInfo.isNullOrEmpty()) return null
            val host = url.host?.lowercase() ?: return null
            val loopback = host == "localhost" || host == "127.0.0.1" || host == "::1"
            val ok = url.protocol == "https" || (url.protocol == "http" && loopback)
            if (!ok) return null
            return raw.trim().trimEnd('/')
        }

        /** True bila base URL menunjuk server di perangkat sendiri (tidak butuh internet). */
        fun isLoopback(raw: String): Boolean {
            val host = runCatching { URL(raw.trim()).host?.lowercase() }.getOrNull() ?: return false
            return host == "localhost" || host == "127.0.0.1" || host == "::1"
        }
    }

    /** Uji koneksi ringan: GET /models. Mengembalikan nama galat singkat bila gagal. */
    suspend fun testConnection(provider: AiProvider, apiKey: String?): String? = withContext(Dispatchers.IO) {
        var conn: HttpURLConnection? = null
        try {
            conn = URL(provider.baseUrl.trimEnd('/') + "/models").openConnection() as HttpURLConnection
            conn.instanceFollowRedirects = false
            conn.requestMethod = "GET"
            conn.connectTimeout = 15_000
            conn.readTimeout = 15_000
            if (!apiKey.isNullOrBlank()) conn.setRequestProperty("Authorization", "Bearer $apiKey")
            val code = conn.responseCode
            if (code in 200..299) null else "HTTP $code"
        } catch (e: Exception) {
            e.message ?: "network"
        } finally {
            runCatching { conn?.disconnect() }
        }
    }
}
