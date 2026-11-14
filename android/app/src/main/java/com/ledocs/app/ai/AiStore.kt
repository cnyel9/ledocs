package com.ledocs.app.ai

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import org.json.JSONArray
import org.json.JSONObject

/** Satu provider AI: nama tampilan, base URL kompatibel OpenAI, dan model bawaan. */
data class AiProvider(
    val id: String,
    val name: String,
    val baseUrl: String,
    val model: String,
    val preset: Boolean = false,
    val keyUrl: String? = null,
    val tag: String? = null,
    val keyOptional: Boolean = false,
)

/** Satu perintah cepat milik pengguna, tampil sebagai chip di layar AI. */
data class AiCommand(val id: String, val label: String, val text: String)

/** Provider siap pakai: tinggal tempel key dari halaman resmi masing masing. Model bawaan bisa diubah. */
object AiPresets {
    val all: List<AiProvider> = listOf(
        AiProvider("openrouter", "OpenRouter", "https://openrouter.ai/api/v1", "deepseek/deepseek-chat:free", preset = true, keyUrl = "https://openrouter.ai/keys", tag = "Gratis"),
        AiProvider("groq", "Groq", "https://api.groq.com/openai/v1", "llama-3.3-70b-versatile", preset = true, keyUrl = "https://console.groq.com/keys", tag = "Gratis"),
        AiProvider("aistudio", "Google AI Studio", "https://generativelanguage.googleapis.com/v1beta/openai/", "gemini-2.5-flash", preset = true, keyUrl = "https://aistudio.google.com/apikey", tag = "Gratis"),
        AiProvider("deepseek", "DeepSeek", "https://api.deepseek.com", "deepseek-chat", preset = true, keyUrl = "https://platform.deepseek.com/api_keys"),
        AiProvider("zai", "Z.AI (GLM)", "https://api.z.ai/api/paas/v4", "glm-4.5-flash", preset = true, keyUrl = "https://z.ai/manage-apikey/apikey-list", tag = "Gratis"),
        AiProvider("cerebras", "Cerebras", "https://api.cerebras.ai/v1", "llama-3.3-70b", preset = true, keyUrl = "https://cloud.cerebras.ai", tag = "Gratis"),
        AiProvider("githubmodels", "GitHub Models", "https://models.github.ai/inference", "openai/gpt-4o-mini", preset = true, keyUrl = "https://github.com/settings/tokens", tag = "Gratis"),
        AiProvider("9router", "9Router (milik sendiri)", "http://127.0.0.1:20128/v1", "oc/deepseek-v4.1-flash-free", preset = true, tag = "Lokal", keyOptional = true),
        AiProvider("ollama", "Ollama (lokal)", "http://127.0.0.1:11434/v1", "llama3.2", preset = true, tag = "Lokal", keyOptional = true),
        AiProvider("lmstudio", "LM Studio (lokal)", "http://127.0.0.1:1234/v1", "local-model", preset = true, tag = "Lokal", keyOptional = true),
    )
}

/**
 * Pengaturan Asisten AI: saklar utama, daftar provider, provider aktif, dan API key.
 * Key tidak pernah disimpan polos: ia dienkripsi AES-256-GCM dengan kunci non-ekspor di
 * Android Keystore, dan hanya dibuka sesaat sebelum request dikirim.
 */
class AiStore(context: Context) {
    private val prefs = context.getSharedPreferences("ai", Context.MODE_PRIVATE)

    // Bukan properti terdelegasi: setter bawaannya akan bentrok signature JVM dengan
    // fun setEnabled(Boolean) di bawah. Getter khusus membaca state, jadi komposisi tetap
    // berlangganan perubahan nilainya.
    private val enabledState = mutableStateOf(prefs.getBoolean("enabled", false))
    val enabled: Boolean get() = enabledState.value
    val providers = mutableStateListOf<AiProvider>()
    var activeId by mutableStateOf(prefs.getString("activeId", null))
        private set

    init {
        val raw = prefs.getString("providers", null)
        if (raw != null) runCatching {
            val arr = JSONArray(raw)
            for (i in 0 until arr.length()) {
                val o = arr.getJSONObject(i)
                providers += AiProvider(
                    id = o.getString("id"),
                    name = o.getString("name"),
                    baseUrl = o.getString("baseUrl"),
                    model = o.getString("model"),
                    preset = o.optBoolean("preset"),
                    keyUrl = o.optString("keyUrl").takeIf { it.isNotBlank() },
                    tag = o.optString("tag").takeIf { it.isNotBlank() },
                    keyOptional = o.optBoolean("keyOptional"),
                )
            }
        }
        if (activeId != null && providers.none { it.id == activeId }) {
            activeId = providers.firstOrNull()?.id
            prefs.edit().putString("activeId", activeId).apply()
        }
    }

    val active: AiProvider? get() = providers.firstOrNull { it.id == activeId } ?: providers.firstOrNull()

    fun setEnabled(on: Boolean) {
        enabledState.value = on
        prefs.edit().putBoolean("enabled", on).apply()
    }

    fun setActive(id: String) {
        activeId = id
        prefs.edit().putString("activeId", id).apply()
    }

    fun upsert(provider: AiProvider) {
        val at = providers.indexOfFirst { it.id == provider.id }
        if (at >= 0) providers[at] = provider else providers += provider
        if (activeId == null) activeId = provider.id
        persist()
    }

    fun remove(id: String) {
        providers.removeAll { it.id == id }
        prefs.edit().remove("key:$id").remove("keys:$id").remove("keyidx:$id").apply()
        if (activeId == id) activeId = providers.firstOrNull()?.id
        persist()
    }

    // ---- Urutan cadangan: rantai provider bila yang aktif gagal -------------------------------

    /** Urutan provider cadangan (id) sesudah provider aktif, sesuai susunan pengguna. */
    var fallbackOrder: List<String>
        get() = runCatching {
            val arr = JSONArray(prefs.getString("fallbackOrder", "[]"))
            (0 until arr.length()).map { arr.getString(it) }.filter { fid -> providers.any { it.id == fid } }
        }.getOrDefault(emptyList())
        set(value) {
            val arr = JSONArray()
            value.forEach { arr.put(it) }
            prefs.edit().putString("fallbackOrder", arr.toString()).apply()
        }

    /** Rantai percobaan lengkap: aktif dulu, lalu cadangan berurutan, hanya yang siap pakai. */
    fun chain(): List<AiProvider> {
        val act = active ?: return emptyList()
        val usable = { p: AiProvider -> p.keyOptional || hasKey(p.id) }
        val out = mutableListOf<AiProvider>()
        if (usable(act)) out += act
        val seen = mutableSetOf(act.id)
        for (fid in fallbackOrder) {
            val p = providers.firstOrNull { it.id == fid } ?: continue
            if (p.id in seen) continue
            seen += p.id
            if (usable(p)) out += p
        }
        for (p in providers) {
            if (p.id in seen) continue
            seen += p.id
            if (usable(p)) out += p
        }
        return out
    }

    // ---- Lencana kemampuan model hasil uji otomatis ------------------------------------------

    fun caps(id: String, model: String): Set<String> =
        prefs.getStringSet("caps:$id:$model", emptySet())?.toSet() ?: emptySet()

    fun setCaps(id: String, model: String, caps: Set<String>) {
        prefs.edit().putStringSet("caps:$id:$model", caps.toSet()).apply()
    }

    // ---- Perintah cepat milik pengguna ---------------------------------------------------------

    val commands = mutableStateListOf<AiCommand>()

    init {
        runCatching {
            val arr = JSONArray(prefs.getString("commands", "[]"))
            for (i in 0 until arr.length()) {
                val o = arr.getJSONObject(i)
                commands += AiCommand(o.getString("id"), o.getString("label"), o.getString("text"))
            }
        }
    }

    fun addCommand(label: String, text: String) {
        commands += AiCommand("cmd-" + java.util.UUID.randomUUID().toString(), label, text)
        persistCommands()
    }

    fun removeCommand(id: String) {
        commands.removeAll { it.id == id }
        persistCommands()
    }

    private fun persistCommands() {
        val arr = JSONArray()
        for (cmd in commands) arr.put(JSONObject().apply { put("id", cmd.id); put("label", cmd.label); put("text", cmd.text) })
        prefs.edit().putString("commands", arr.toString()).apply()
    }

    private fun persist() {
        val arr = JSONArray()
        for (p in providers) {
            arr.put(JSONObject().apply {
                put("id", p.id); put("name", p.name); put("baseUrl", p.baseUrl); put("model", p.model)
                put("preset", p.preset); put("keyUrl", p.keyUrl ?: ""); put("tag", p.tag ?: ""); put("keyOptional", p.keyOptional)
            })
        }
        prefs.edit().putString("providers", arr.toString()).putString("activeId", activeId).apply()
    }

    // ---- API key: brankas Keystore -----------------------------------------------------------
    // Satu provider boleh menyimpan banyak key (rotasi saat kuota habis). Format simpan:
    // "keys:<id>" berisi JSONArray string terenkripsi; format lama "key:<id>" (satu string)
    // dibaca sebagai daftar berisi satu key dan bermigrasi saat daftar ditulis ulang.

    private fun packedKeys(id: String): List<String> {
        val raw = prefs.getString("keys:$id", null)
        if (raw != null) return runCatching {
            val arr = JSONArray(raw)
            (0 until arr.length()).map { arr.getString(it) }
        }.getOrDefault(emptyList())
        val legacy = prefs.getString("key:$id", null) ?: return emptyList()
        return listOf(legacy)
    }

    private fun storePackedKeys(id: String, packed: List<String>) {
        val arr = JSONArray()
        packed.forEach { arr.put(it) }
        prefs.edit().putString("keys:$id", arr.toString()).remove("key:$id").apply()
    }

    fun keyCount(id: String): Int = keys(id).size

    fun hasKey(id: String): Boolean = keyCount(id) > 0

    /**
     * Semua key provider ini dalam keadaan terbuka. Daftar terenkripsi menyembuhkan diri:
     * entri yang gagal dibuka dibuang dari penyimpanan, jadi hitungan, rotasi, dan penghapusan
     * selalu menunjuk daftar yang sama persis.
     */
    fun keys(id: String): List<String> {
        val packed = packedKeys(id)
        if (packed.isEmpty()) return emptyList()
        val keptPacked = mutableListOf<String>()
        val out = mutableListOf<String>()
        for (p in packed) {
            val plain = decryptPacked(p)
            if (plain != null) {
                keptPacked += p
                out += plain
            }
        }
        if (keptPacked.size != packed.size) storePackedKeys(id, keptPacked)
        return out
    }

    private fun decryptPacked(stored: String): String? {
        val parts = stored.split(":")
        if (parts.size != 3 || parts[0] != "v1") return null
        return try {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, vaultKey(), GCMParameterSpec(128, Base64.decode(parts[1], Base64.NO_WRAP)))
            String(cipher.doFinal(Base64.decode(parts[2], Base64.NO_WRAP)), Charsets.UTF_8)
        } catch (e: KeyPermanentlyInvalidatedException) {
            // Kunci brankas gugur permanen (mis. kunci layar perangkat berubah total): semua key
            // tersimpan ikut gugur, pengguna memasukkan ulang. Jangan biarkan status berbohong.
            resetVault()
            null
        } catch (e: Exception) {
            null
        }
    }

    /** Key yang sedang giliran dipakai provider ini (rotasi menunjuk ke sini). */
    fun key(id: String): String? {
        val all = keys(id)
        if (all.isEmpty()) return null
        val idx = prefs.getInt("keyidx:$id", 0)
        return all[((idx % all.size) + all.size) % all.size]
    }

    /** Memajukan giliran key provider ini dan mengembalikan key berikutnya (null bila kosong). */
    fun rotateKey(id: String): String? {
        val all = keys(id)
        if (all.size < 2) return all.firstOrNull()
        val next = (prefs.getInt("keyidx:$id", 0) + 1) % all.size
        prefs.edit().putInt("keyidx:$id", next).apply()
        return all[next]
    }

    /** Mengganti seluruh key provider ini dengan satu key. False bila brankas tidak bisa dipakai. */
    fun setKey(id: String, key: String): Boolean {
        if (key.isBlank()) {
            prefs.edit().remove("key:$id").remove("keys:$id").remove("keyidx:$id").apply()
            return true
        }
        val packed = encryptPacked(key) ?: return false
        storePackedKeys(id, listOf(packed))
        prefs.edit().remove("keyidx:$id").apply()
        return true
    }

    /** Menambah satu key ke daftar provider ini. False bila brankas tidak bisa dipakai. */
    fun addKey(id: String, key: String): Boolean {
        if (key.isBlank()) return true
        val packed = encryptPacked(key) ?: return false
        storePackedKeys(id, packedKeys(id) + packed)
        return true
    }

    /** Menghapus key ke-[index] (urutan tampilan) dari provider ini. */
    fun removeKeyAt(id: String, index: Int) {
        keys(id) // sembuhkan daftar dulu supaya indeks packed sejajar dengan urutan tampilan
        val packed = packedKeys(id).toMutableList()
        if (index !in packed.indices) return
        packed.removeAt(index)
        storePackedKeys(id, packed)
        prefs.edit().remove("keyidx:$id").apply()
    }

    private fun encryptPacked(key: String): String? {
        return try {
            encryptOnce(key)
        } catch (e: KeyPermanentlyInvalidatedException) {
            resetVault()
            runCatching { encryptOnce(key) }.getOrNull()
        } catch (e: Exception) {
            null
        }
    }

    private fun encryptOnce(key: String): String {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, vaultKey())
        return "v1:" + Base64.encodeToString(cipher.iv, Base64.NO_WRAP) + ":" +
            Base64.encodeToString(cipher.doFinal(key.toByteArray(Charsets.UTF_8)), Base64.NO_WRAP)
    }

    /** Menghapus kunci brankas dan semua ciphertext key (dipakai sesudah invalidasi permanen). */
    private fun resetVault() {
        runCatching {
            val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            ks.deleteEntry("ledocs.ai.vault.v1")
        }
        val edit = prefs.edit()
        for (k in prefs.all.keys) if (k.startsWith("key:") || k.startsWith("keys:") || k.startsWith("keyidx:")) edit.remove(k)
        edit.apply()
    }

    @Synchronized
    private fun vaultKey(): SecretKey {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (ks.getEntry("ledocs.ai.vault.v1", null) as? KeyStore.SecretKeyEntry)?.let { return it.secretKey }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        gen.init(
            KeyGenParameterSpec.Builder("ledocs.ai.vault.v1", KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setRandomizedEncryptionRequired(true)
                .build(),
        )
        return gen.generateKey()
    }

    // ---- Persetujuan pengiriman: per provider + host + dokumen -------------------------------

    private fun consentKey(providerId: String, host: String, docKey: String?): String =
        "$providerId|$host|${docKey ?: "bebas"}"

    fun consented(providerId: String, host: String, docKey: String?): Boolean =
        prefs.getStringSet("consented", emptySet())?.contains(consentKey(providerId, host, docKey)) == true

    fun consent(providerId: String, host: String, docKey: String?) {
        val now = (prefs.getStringSet("consented", emptySet()) ?: emptySet()).toMutableSet()
        now += consentKey(providerId, host, docKey)
        prefs.edit().putStringSet("consented", now).apply()
    }

    /** Menghapus semua data AI: provider, key terenkripsi, persetujuan, dan saklar. */
    fun wipeAll() {
        providers.clear()
        commands.clear()
        activeId = null
        enabledState.value = false
        prefs.edit().clear().apply()
    }
}
