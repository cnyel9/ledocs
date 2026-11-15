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
     * Mengirim [messages] dan mengalirkan teks jawaban ke [onDelta] serta teks penalaran model
     * (bila ada) ke [onReasoning]. Mengembalikan pesan asisten lengkap sebagai JSONObject.
     * Retry sampai [MAX_ATTEMPTS] kali selama belum ada token yang mengalir (jaringan lambat,
     * 429, 5xx; menghormati Retry-After). Sekali token mengalir, kegagalan di tengah dilaporkan
     * apa adanya supaya lapisan sesi bisa menawarkan tombol Lanjutkan, bukan mengulang diam diam.
     */
    suspend fun chatStream(
        provider: AiProvider,
        apiKey: String?,
        messages: JSONArray,
        temperature: Double = 0.7,
        onReasoning: (String) -> Unit = {},
        onRetry: (attempt: Int) -> Unit = {},
        onDelta: (String) -> Unit,
    ): JSONObject = withContext(Dispatchers.IO) {
        val body = JSONObject().apply {
            put("model", provider.model)
            put("messages", messages)
            put("stream", true)
            put("temperature", temperature)
        }.toString()
        var emitted = false
        val delta: (String) -> Unit = { piece -> emitted = true; onDelta(piece) }
        val reasoning: (String) -> Unit = { piece -> emitted = true; onReasoning(piece) }
        var attempt = 0
        var lastError: AiException? = null
        while (attempt < MAX_ATTEMPTS) {
            attempt++
            try {
                return@withContext streamOnce(provider, apiKey, body, reasoning, delta)
            } catch (e: CancellationException) {
                throw e
            } catch (e: AiException) {
                lastError = e
                val retryable = !emitted && (e.kind == AiException.Kind.QUOTA || e.kind == AiException.Kind.SERVER || e.kind == AiException.Kind.NETWORK)
                if (!retryable || attempt >= MAX_ATTEMPTS) throw e
                // Kabari lapisan sesi SEBELUM menunggu: pengulangan tidak boleh diam di layar.
                onRetry(attempt + 1)
                val waitMs = e.retryAfterSec?.let { minOf(it * 1000, 15_000L) } ?: minOf(1000L shl (attempt - 1), 8000L)
                delay(waitMs)
            }
        }
        throw lastError ?: AiException(AiException.Kind.NETWORK, "network")
    }

    private suspend fun streamOnce(
        provider: AiProvider,
        apiKey: String?,
        body: String,
        onReasoning: (String) -> Unit,
        onDelta: (String) -> Unit,
    ): JSONObject {
        val url = try {
            URL(provider.baseUrl.trimEnd('/') + "/chat/completions")
        } catch (e: Exception) {
            throw AiException(AiException.Kind.NETWORK, e.message ?: "bad url")
        }
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
            if (code !in 200..299) throw httpError(conn, code)
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
                if (rPiece.isNotEmpty()) {
                    reasoning.append(rPiece)
                    onReasoning(rPiece)
                }
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

    /** Satu panggilan tanpa streaming (untuk ringkasan otomatis dan uji kemampuan). */
    suspend fun complete(
        provider: AiProvider,
        apiKey: String?,
        messages: JSONArray,
        temperature: Double = 0.7,
        tools: JSONArray? = null,
    ): JSONObject = withContext(Dispatchers.IO) {
        val body = JSONObject().apply {
            put("model", provider.model)
            put("messages", messages)
            put("stream", false)
            put("temperature", temperature)
            if (tools != null) put("tools", tools)
        }.toString()
        val url = try {
            URL(provider.baseUrl.trimEnd('/') + "/chat/completions")
        } catch (e: Exception) {
            throw AiException(AiException.Kind.NETWORK, e.message ?: "bad url")
        }
        val conn = url.openConnection() as HttpURLConnection
        try {
            conn.requestMethod = "POST"
            conn.instanceFollowRedirects = false
            conn.connectTimeout = 30_000
            conn.readTimeout = 120_000
            conn.doOutput = true
            conn.setRequestProperty("Content-Type", "application/json")
            if (!apiKey.isNullOrBlank()) conn.setRequestProperty("Authorization", "Bearer $apiKey")
            conn.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
            val code = conn.responseCode
            if (code !in 200..299) throw httpError(conn, code)
            val text = conn.inputStream.bufferedReader().readText()
            val msg = JSONObject(text).optJSONArray("choices")?.optJSONObject(0)?.optJSONObject("message")
                ?: throw AiException(AiException.Kind.SERVER, "empty response")
            JSONObject().apply {
                put("role", "assistant")
                put("content", msg.optString("content"))
                val r = msg.optString("reasoning_content")
                if (r.isNotEmpty()) put("reasoning_content", r)
                msg.optJSONArray("tool_calls")?.let { put("tool_calls", it) }
            }
        } catch (e: AiException) {
            throw e
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            throw AiException(AiException.Kind.NETWORK, e.message ?: "network")
        } finally {
            runCatching { conn.disconnect() }
        }
    }

    /**
     * Uji kemampuan model dengan maksimal tiga panggilan kecil: teks + tool, lalu (bila perlu)
     * teks polos, dan satu gambar piksel untuk vision. Hasilnya himpunan label kemampuan:
     * "teks", "berpikir", "tool", "gambar". Kegagalan total mengembalikan himpunan kosong.
     */
    suspend fun probeCapabilities(provider: AiProvider, apiKey: String?): Set<String> {
        val caps = mutableSetOf<String>()
        val toolDef = JSONArray().put(
            JSONObject().apply {
                put("type", "function")
                put("function", JSONObject().apply {
                    put("name", "get_weather")
                    put("description", "Melihat cuaca sebuah kota")
                    put("parameters", JSONObject().apply {
                        put("type", "object")
                        put("properties", JSONObject().put("city", JSONObject().put("type", "string")))
                        put("required", JSONArray().put("city"))
                    })
                })
            },
        )
        val toolMessages = JSONArray().put(
            JSONObject().put("role", "user").put("content", "Gunakan tool get_weather untuk mengecek cuaca Jakarta."),
        )
        var answered = runCatching { complete(provider, apiKey, toolMessages, 0.0, toolDef) }.getOrNull()
        if (answered != null) {
            caps += "teks"
            if (answered.has("tool_calls")) caps += "tool"
            if (answered.optString("reasoning_content").isNotBlank()) caps += "berpikir"
        } else {
            val plain = JSONArray().put(JSONObject().put("role", "user").put("content", "Balas dengan kata OK."))
            answered = runCatching { complete(provider, apiKey, plain, 0.0) }.getOrNull()
            if (answered != null) {
                caps += "teks"
                if (answered.optString("reasoning_content").isNotBlank()) caps += "berpikir"
            }
        }
        if ("teks" in caps) {
            val visionMessages = JSONArray().put(
                JSONObject().put("role", "user").put(
                    "content",
                    JSONArray()
                        .put(JSONObject().put("type", "text").put("text", "Warna apa gambar ini? Jawab satu kata."))
                        .put(
                            JSONObject().put("type", "image_url").put(
                                "image_url",
                                JSONObject().put("url", "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg=="),
                            ),
                        ),
                ),
            )
            if (runCatching { complete(provider, apiKey, visionMessages, 0.0) }.getOrNull() != null) caps += "gambar"
        }
        return caps
    }

    private fun httpError(conn: HttpURLConnection, code: Int): AiException {
        val errBody = runCatching { conn.errorStream?.bufferedReader()?.readText() }.getOrNull().orEmpty()
        val msg = runCatching { JSONObject(errBody).optJSONObject("error")?.optString("message") }.getOrNull()?.takeIf { it.isNotBlank() } ?: "HTTP $code"
        val retryAfter = conn.getHeaderField("Retry-After")?.toLongOrNull()
        return AiException(
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

    companion object {
        /** Batas percobaan ulang otomatis untuk jaringan lambat (hanya sebelum token pertama). */
        const val MAX_ATTEMPTS = 10

        /**
         * Validasi base URL provider: wajib https, kecuali host loopback persis (localhost,
         * 127.0.0.1, ::1) yang boleh http untuk server lokal pengguna sendiri. Tanpa userinfo.
         * Mengembalikan URL ternormalisasi (tanpa garis miring akhir) atau null bila tidak sah.
         */
        fun validateBaseUrl(raw: String): String? {
            val url = runCatching { URL(raw.trim()) }.getOrNull() ?: return null
            if (!url.userInfo.isNullOrEmpty()) return null
            val host = url.host?.lowercase()?.removeSurrounding("[", "]") ?: return null
            val loopback = host == "localhost" || host == "127.0.0.1" || host == "::1"
            val ok = url.protocol == "https" || (url.protocol == "http" && loopback)
            if (!ok) return null
            return raw.trim().trimEnd('/')
        }

        /** True bila base URL menunjuk server di perangkat sendiri (tidak butuh internet). */
        fun isLoopback(raw: String): Boolean {
            val host = runCatching { URL(raw.trim()).host?.lowercase()?.removeSurrounding("[", "]") }.getOrNull() ?: return false
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
