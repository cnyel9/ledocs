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
        prefs.edit().remove("key:$id").apply()
        if (activeId == id) activeId = providers.firstOrNull()?.id
        persist()
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

    fun hasKey(id: String): Boolean = prefs.getString("key:$id", null) != null

    fun key(id: String): String? {
        val stored = prefs.getString("key:$id", null) ?: return null
        val parts = stored.split(":")
        if (parts.size != 3 || parts[0] != "v1") {
            prefs.edit().remove("key:$id").apply()
            return null
        }
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
            // Ciphertext rusak atau tidak terbaca: hapus supaya hasKey tidak berbohong.
            prefs.edit().remove("key:$id").apply()
            null
        }
    }

    /** Menyimpan key terenkripsi. Mengembalikan false bila brankas perangkat tidak bisa dipakai. */
    fun setKey(id: String, key: String): Boolean {
        if (key.isBlank()) {
            prefs.edit().remove("key:$id").apply()
            return true
        }
        return try {
            encryptAndStore(id, key)
        } catch (e: KeyPermanentlyInvalidatedException) {
            resetVault()
            runCatching { encryptAndStore(id, key) }.getOrDefault(false)
        } catch (e: Exception) {
            false
        }
    }

    private fun encryptAndStore(id: String, key: String): Boolean {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, vaultKey())
        val packed = "v1:" + Base64.encodeToString(cipher.iv, Base64.NO_WRAP) + ":" +
            Base64.encodeToString(cipher.doFinal(key.toByteArray(Charsets.UTF_8)), Base64.NO_WRAP)
        prefs.edit().putString("key:$id", packed).apply()
        return true
    }

    /** Menghapus kunci brankas dan semua ciphertext key (dipakai sesudah invalidasi permanen). */
    private fun resetVault() {
        runCatching {
            val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            ks.deleteEntry("ledocs.ai.vault.v1")
        }
        val edit = prefs.edit()
        for (k in prefs.all.keys) if (k.startsWith("key:")) edit.remove(k)
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

    private fun consentKey(providerId: String, host: String, docId: ULong?): String =
        "$providerId|$host|${docId?.toString() ?: "bebas"}"

    fun consented(providerId: String, host: String, docId: ULong?): Boolean =
        prefs.getStringSet("consented", emptySet())?.contains(consentKey(providerId, host, docId)) == true

    fun consent(providerId: String, host: String, docId: ULong?) {
        val now = (prefs.getStringSet("consented", emptySet()) ?: emptySet()).toMutableSet()
        now += consentKey(providerId, host, docId)
        prefs.edit().putStringSet("consented", now).apply()
    }

    /** Menghapus semua data AI: provider, key terenkripsi, persetujuan, dan saklar. */
    fun wipeAll() {
        providers.clear()
        activeId = null
        enabledState.value = false
        prefs.edit().clear().apply()
    }
}
