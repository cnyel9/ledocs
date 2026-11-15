package com.ledocs.app.update

import android.content.Context
import android.content.Intent
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.content.FileProvider
import com.ledocs.app.R
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

/** Satu rilis baru yang ditemukan di GitHub Releases. */
data class UpdateInfo(
    val version: String,
    val tag: String,
    val notes: String,
    val apkUrl: String,
    val apkName: String,
    val sizeBytes: Long,
    val shaUrl: String?,
    val htmlUrl: String,
)

/**
 * Pemeriksa pembaruan LeDocs, pola yang sama dengan LePlay: sumber kebenaran
 * adalah halaman rilis GitHub, tanpa server dan tanpa akun. Cek otomatis
 * dibatasi paling sering 12 jam sekali dan kegagalannya diam (limit GitHub
 * per IP mudah habis di jaringan CGNAT); cek manual dari Pengaturan memberi
 * kabar lewat [onMessage]. Unduhan diverifikasi berlapis sebelum pemasang
 * sistem dipanggil: ukuran aset, SHA-256 dari aset pendamping .sha256 (bila
 * rilis memilikinya), dan digest sertifikat penandatangan harus sama persis
 * dengan aplikasi yang terpasang.
 */
class UpdateState(
    private val scope: CoroutineScope,
    private val app: Context,
    private val onMessage: (Int) -> Unit,
) {
    var info by mutableStateOf<UpdateInfo?>(null)
        private set
    var visible by mutableStateOf(false)
        private set
    var checking by mutableStateOf(false)
        private set
    var downloading by mutableStateOf(false)
        private set
    var progress by mutableFloatStateOf(0f)
        private set
    var errorRes by mutableStateOf<Int?>(null)
        private set
    var downloaded by mutableStateOf<File?>(null)
        private set

    private val prefs = app.getSharedPreferences("updates", Context.MODE_PRIVATE)
    private var downloadJob: Job? = null

    private fun installedCode(): Long = try {
        app.packageManager.getPackageInfo(app.packageName, 0).longVersionCode
    } catch (e: Exception) {
        0L
    }

    private fun installedName(): String = try {
        app.packageManager.getPackageInfo(app.packageName, 0).versionName ?: "?"
    } catch (e: Exception) {
        "?"
    }

    /**
     * Skema kode versi yang sama dengan android/app/build.gradle.kts: basis
     * dari semver (rilis final memakai 99), kali 10 plus kode ABI arm64-v8a
     * (2), karena LeDocs hanya menerbitkan satu APK arm64.
     */
    private fun codeForVersion(version: String): Long? {
        val parts = version.substringBefore('-').split('.')
        if (parts.size != 3) return null
        val nums = parts.map { it.toIntOrNull() ?: return null }
        if (nums.any { it < 0 || it >= 100 }) return null
        val base = ((nums[0] * 100 + nums[1]) * 100 + nums[2]) * 100 + 99
        return base * 10L + 2L
    }

    fun checkAuto() {
        val last = prefs.getLong("lastCheck", 0L)
        if (System.currentTimeMillis() - last < AUTO_INTERVAL_MS) return
        check(manual = false)
    }

    fun checkManual() = check(manual = true)

    private fun check(manual: Boolean) {
        if (checking) return
        checking = true
        scope.launch {
            val latest = withContext(Dispatchers.IO) { runCatching { fetchLatest() }.getOrNull() }
            checking = false
            prefs.edit().putLong("lastCheck", System.currentTimeMillis()).apply()
            if (latest == null) {
                if (manual) onMessage(R.string.update_check_failed)
                return@launch
            }
            val latestCode = codeForVersion(latest.version)
            if (latestCode != null && latestCode > installedCode()) {
                info = latest
                errorRes = null
                downloaded = null
                val dismissed = prefs.getString("dismissedTag", null)
                if (manual || dismissed != latest.tag) visible = true
            } else if (manual) {
                onMessage(R.string.update_uptodate)
            }
        }
    }

    fun dismiss() {
        if (downloading) return
        visible = false
        info?.let { prefs.edit().putString("dismissedTag", it.tag).apply() }
    }

    fun download() {
        val update = info ?: return
        if (downloading) return
        downloading = true
        errorRes = null
        progress = 0f
        downloadJob = scope.launch {
            val file = withContext(Dispatchers.IO) {
                runCatching { downloadAndVerify(update) { p -> progress = p } }.getOrNull()
            }
            downloading = false
            downloadJob = null
            if (file != null) {
                downloaded = file
                install()
            } else {
                errorRes = R.string.update_download_failed
            }
        }
    }

    fun cancelDownload() {
        downloadJob?.cancel()
        downloadJob = null
        downloading = false
        progress = 0f
    }

    fun install() {
        val file = downloaded ?: return
        if (!app.packageManager.canRequestPackageInstalls()) {
            errorRes = R.string.update_allow_installs
            runCatching {
                app.startActivity(
                    Intent(
                        Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                        Uri.parse("package:${app.packageName}"),
                    ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                )
            }
            return
        }
        errorRes = null
        runCatching {
            val uri = FileProvider.getUriForFile(app, "${app.packageName}.files", file)
            app.startActivity(
                Intent(Intent.ACTION_VIEW).apply {
                    setDataAndType(uri, "application/vnd.android.package-archive")
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
                },
            )
        }.onFailure { errorRes = R.string.update_installer_failed }
    }

    // ---- jaringan (hanya dipanggil dari Dispatchers.IO) -------------------

    private data class Attempt(val info: UpdateInfo?, val retryable: Boolean, val retryAfterMs: Long? = null)

    private fun fetchLatest(): UpdateInfo? {
        var attempt = 0
        while (true) {
            val r = fetchLatestOnce()
            if (r.info != null || !r.retryable || attempt >= 2) return r.info
            val wait = (r.retryAfterMs ?: (1500L shl attempt)).coerceIn(500L, 60_000L)
            runCatching { Thread.sleep(wait) }
            attempt++
        }
    }

    private fun open(url: String): HttpURLConnection =
        (URL(url).openConnection() as HttpURLConnection).apply {
            connectTimeout = 15_000
            readTimeout = 60_000
            setRequestProperty("Accept", "application/vnd.github+json")
            setRequestProperty("User-Agent", "LeDocs/${installedName()}")
        }

    private fun fetchLatestOnce(): Attempt {
        val conn = try {
            open(LATEST_URL)
        } catch (e: Exception) {
            return Attempt(null, retryable = true)
        }
        return try {
            val code = conn.responseCode
            if (code != 200) {
                val retryable = code == 403 || code == 429 || code >= 500
                val retryAfter = conn.getHeaderField("Retry-After")?.toLongOrNull()?.times(1000L)
                return Attempt(null, retryable = retryable, retryAfterMs = retryAfter)
            }
            val raw = conn.inputStream.bufferedReader().use { it.readText() }
            val json = JSONObject(raw)
            val tag = json.optString("tag_name")
            if (tag.isBlank()) return Attempt(null, retryable = false)
            val version = tag.removePrefix("v").removePrefix("V").trim()
            if (version.isBlank() || codeForVersion(version) == null) return Attempt(null, retryable = false)
            val assets = json.optJSONArray("assets") ?: return Attempt(null, retryable = false)
            var apkName: String? = null
            var apkUrl: String? = null
            var size = 0L
            var shaUrl: String? = null
            for (i in 0 until assets.length()) {
                val asset = assets.optJSONObject(i) ?: continue
                val name = asset.optString("name")
                if (name.endsWith(".apk", ignoreCase = true)) {
                    apkName = name
                    apkUrl = asset.optString("browser_download_url")
                    size = asset.optLong("size")
                }
            }
            val name = apkName ?: return Attempt(null, retryable = false)
            val url = apkUrl?.ifBlank { null } ?: return Attempt(null, retryable = false)
            for (i in 0 until assets.length()) {
                val asset = assets.optJSONObject(i) ?: continue
                if (asset.optString("name") == "$name.sha256") {
                    shaUrl = asset.optString("browser_download_url").ifBlank { null }
                }
            }
            Attempt(
                UpdateInfo(
                    version = version,
                    tag = tag,
                    notes = json.optString("body").orEmpty(),
                    apkUrl = url,
                    apkName = name,
                    sizeBytes = size,
                    shaUrl = shaUrl,
                    htmlUrl = json.optString("html_url").orEmpty(),
                ),
                retryable = false,
            )
        } catch (e: Exception) {
            Attempt(null, retryable = true)
        } finally {
            conn.disconnect()
        }
    }

    private fun downloadAndVerify(update: UpdateInfo, onProgress: (Float) -> Unit): File {
        val expectedSha = update.shaUrl?.let { url ->
            val conn = open(url)
            try {
                if (conn.responseCode != 200) null
                else conn.inputStream.bufferedReader().use { it.readText() }
                    .trim().split(Regex("\\s+")).firstOrNull()?.lowercase()
            } finally {
                conn.disconnect()
            }
        }
        val dir = File(app.cacheDir, "updates").apply { mkdirs() }
        // Unduh ke berkas.part unik dulu supaya unduhan yang dibatalkan lalu
        // diulang tidak pernah berbagi berkas yang sama dengan unduhan lama.
        val part = File(dir, update.apkName + "." + System.currentTimeMillis() + ".part")
        val conn = open(update.apkUrl)
        try {
            if (conn.responseCode != 200) throw IllegalStateException("HTTP ${conn.responseCode}")
            val total = conn.contentLengthLong.takeIf { it > 0 } ?: update.sizeBytes
            conn.inputStream.use { input ->
                part.outputStream().use { output ->
                    val buffer = ByteArray(64 * 1024)
                    var done = 0L
                    var lastTick = 0L
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        output.write(buffer, 0, read)
                        done += read
                        if (total > 0) {
                            val now = System.currentTimeMillis()
                            if (now - lastTick > 80 || done >= total) {
                                lastTick = now
                                onProgress((done.toFloat() / total).coerceIn(0f, 1f))
                            }
                        }
                    }
                }
            }
        } catch (e: Exception) {
            part.delete()
            throw e
        } finally {
            conn.disconnect()
        }
        val target = File(dir, update.apkName)
        if (target.exists()) target.delete()
        if (!part.renameTo(target)) {
            part.copyTo(target, overwrite = true)
            part.delete()
        }
        // Verifikasi berlapis; satu saja gagal, berkas dibuang dan unduhan batal.
        fun fail(reason: String): Nothing {
            target.delete()
            throw IllegalStateException(reason)
        }
        if (target.length() <= 0L) fail("empty")
        if (update.sizeBytes > 0 && target.length() != update.sizeBytes) fail("size")
        if (expectedSha != null && sha256Hex(target) != expectedSha) fail("sha256")
        val archive = archiveSigners(target) ?: fail("unsigned")
        if (archive != installedSigners()) fail("certificate")
        return target
    }

    private fun sha256Hex(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(64 * 1024)
            while (true) {
                val read = input.read(buffer)
                if (read < 0) break
                digest.update(buffer, 0, read)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }

    private fun signerDigests(info: PackageInfo): Set<String> {
        val signing = info.signingInfo ?: return emptySet()
        val signers = if (signing.hasMultipleSigners()) {
            signing.apkContentsSigners
        } else {
            signing.signingCertificateHistory?.takeIf { it.isNotEmpty() } ?: signing.apkContentsSigners
        }
        return signers.map { cert ->
            MessageDigest.getInstance("SHA-256").digest(cert.toByteArray())
                .joinToString("") { "%02x".format(it) }
        }.toSet()
    }

    private fun installedSigners(): Set<String> = try {
        signerDigests(app.packageManager.getPackageInfo(app.packageName, PackageManager.GET_SIGNING_CERTIFICATES))
    } catch (e: Exception) {
        emptySet()
    }

    private fun archiveSigners(file: File): Set<String>? = try {
        val info = app.packageManager.getPackageArchiveInfo(
            file.absolutePath,
            PackageManager.GET_SIGNING_CERTIFICATES,
        ) ?: return null
        signerDigests(info).takeIf { it.isNotEmpty() }
    } catch (e: Exception) {
        null
    }

    companion object {
        private const val LATEST_URL = "https://api.github.com/repos/cnyel9/ledocs/releases/latest"
        private const val AUTO_INTERVAL_MS = 12 * 60 * 60 * 1000L
    }
}
