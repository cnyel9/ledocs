package com.ledocs.app.ui.desk

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsBottomHeight
import androidx.compose.foundation.layout.windowInsetsTopHeight
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.ledocs.app.R
import com.ledocs.app.ai.AiClient
import com.ledocs.app.ai.AiPresets
import com.ledocs.app.ai.AiProvider
import com.ledocs.app.ui.AppViewModel
import com.ledocs.app.ui.LucideIcons
import com.ledocs.app.ui.glass.GlassDialog
import com.ledocs.app.ui.glass.GlassPill
import com.ledocs.app.ui.glass.GlassTextField
import com.ledocs.app.ui.theme.Candy
import com.ledocs.app.ui.theme.ClearType
import com.ledocs.app.ui.theme.clear
import java.util.UUID
import kotlinx.coroutines.launch

/** Pengaturan Asisten AI: saklar utama, provider siap pakai, provider kustom, dan API key. */
@Composable
fun AiSettingsPage(vm: AppViewModel, onBack: () -> Unit) {
    val c = clear
    val store = vm.aiStore
    var editing by remember { mutableStateOf<AiProvider?>(null) }
    var creating by remember { mutableStateOf(false) }
    var confirmWipe by remember { mutableStateOf(false) }
    var orderIds by remember { mutableStateOf<List<String>?>(null) }
    val draft = remember { AiProvider(id = "custom-" + UUID.randomUUID().toString(), name = "", baseUrl = "", model = "") }
    // Dibaca di badan composable (bukan di dalam DSL LazyColumn) supaya perubahan provider
    // memicu recomposition dengan benar.
    val providersNow = store.providers
    val missing = AiPresets.all.filter { preset -> providersNow.none { it.id == preset.id } }
    LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(bottom = 32.dp)) {
        item { Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars)) }
        item {
            Spacer(Modifier.height(14.dp))
            ScreenHeader(null, stringResource(R.string.ai_providers_title), onBack = onBack)
            Text(stringResource(R.string.ai_providers_body), style = ClearType.Body, color = c.inkSoft, modifier = Modifier.padding(horizontal = 30.dp, vertical = 10.dp))
        }
        item { Section(stringResource(R.string.ai_title)) }
        item {
            Panel {
                ToggleRow(LucideIcons.MessageSquareText, Candy.Sky, stringResource(R.string.ai_toggle_label), stringResource(R.string.ai_settings_body), store.enabled, first = true) { store.setEnabled(it) }
            }
        }
        if (providersNow.isNotEmpty()) {
            item { Section(stringResource(R.string.ai_providers_row)) }
            item {
                Panel {
                    // Label lencana diambil dulu di konteks composable: stringResource tidak
                    // boleh dipanggil dari dalam lambda joinToString/buildString.
                    val capTeks = stringResource(R.string.ai_cap_teks)
                    val capGambar = stringResource(R.string.ai_cap_gambar)
                    val capBerpikir = stringResource(R.string.ai_cap_berpikir)
                    val capTool = stringResource(R.string.ai_cap_tool)
                    providersNow.forEachIndexed { i, p ->
                        val capsNow = store.caps(p.id, p.model)
                        val status = buildString {
                            if (store.active?.id == p.id) append(stringResource(R.string.ai_provider_active)).append(" · ")
                            append(
                                when {
                                    p.keyOptional -> stringResource(R.string.ai_provider_no_key_needed)
                                    store.keyCount(p.id) > 1 -> stringResource(R.string.ai_keys_count, store.keyCount(p.id))
                                    store.hasKey(p.id) -> stringResource(R.string.ai_provider_key_saved)
                                    else -> stringResource(R.string.ai_provider_key_missing)
                                },
                            )
                            if (capsNow.isNotEmpty()) {
                                append(" · ")
                                append(capsNow.joinToString(", ") { cap ->
                                    when (cap) {
                                        "teks" -> capTeks
                                        "gambar" -> capGambar
                                        "berpikir" -> capBerpikir
                                        "tool" -> capTool
                                        else -> cap
                                    }
                                })
                            }
                        }
                        SettingRow(LucideIcons.Globe, Candy.Sky, p.name, { editing = p }, value = status, chevron = true, first = i == 0)
                    }
                }
            }
        }
        if (providersNow.size > 1) {
            item { Section(stringResource(R.string.ai_fallback_title)) }
            item {
                Column {
                    Text(
                        stringResource(R.string.ai_fallback_body),
                        style = ClearType.Body, color = c.inkSoft,
                        modifier = Modifier.padding(horizontal = 30.dp),
                    )
                    Spacer(Modifier.height(10.dp))
                    Panel {
                    val baseOrder = orderIds ?: store.fallbackOrder
                    val effectiveOrder = (baseOrder.filter { id -> providersNow.any { it.id == id } } +
                        providersNow.map { it.id }.filter { it !in baseOrder }).distinct()
                    effectiveOrder.forEachIndexed { i, pid ->
                        val p = providersNow.first { it.id == pid }
                        Row(
                            Modifier.fillMaxWidth().padding(vertical = 10.dp),
                            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                        ) {
                            Column(Modifier.weight(1f)) {
                                Text(p.name, style = ClearType.BodyStrong, color = c.ink)
                                Text(
                                    if (store.active?.id == p.id) stringResource(R.string.ai_provider_active) else p.model,
                                    style = ClearType.Caption, color = c.inkSoft,
                                )
                            }
                            if (i > 0) {
                                GlassPill(
                                    onClick = {
                                        val next = effectiveOrder.toMutableList()
                                        next.add(i - 1, next.removeAt(i))
                                        orderIds = next
                                        store.fallbackOrder = next
                                    },
                                    height = 34.dp, label = stringResource(R.string.ai_move_up),
                                ) { Text(stringResource(R.string.ai_move_up), style = ClearType.CaptionStrong, color = c.ink) }
                                Spacer(Modifier.width(8.dp))
                            }
                            if (i < effectiveOrder.lastIndex) {
                                GlassPill(
                                    onClick = {
                                        val next = effectiveOrder.toMutableList()
                                        next.add(i + 1, next.removeAt(i))
                                        orderIds = next
                                        store.fallbackOrder = next
                                    },
                                    height = 34.dp, label = stringResource(R.string.ai_move_down),
                                ) { Text(stringResource(R.string.ai_move_down), style = ClearType.CaptionStrong, color = c.ink) }
                            }
                        }
                    }
                }
                }
            }
        }
        if (store.commands.isNotEmpty()) {
            item { Section(stringResource(R.string.ai_commands_title)) }
            item {
                Panel {
                    store.commands.forEachIndexed { i, cmd ->
                        SettingRow(LucideIcons.MessageSquare, Candy.Sky, cmd.label, { store.removeCommand(cmd.id) }, value = cmd.text, first = i == 0)
                    }
                }
            }
        }
        if (missing.isNotEmpty()) {
            item { Section(stringResource(R.string.ai_provider_add_preset)) }
            item {
                Panel {
                    missing.forEachIndexed { i, preset ->
                        SettingRow(
                            LucideIcons.Plus, Candy.Mint,
                            preset.name + (preset.tag?.let { " · $it" } ?: ""),
                            { store.upsert(preset); editing = preset },
                            value = preset.model, chevron = true, first = i == 0,
                        )
                    }
                }
            }
        }
        item { Section(stringResource(R.string.ai_provider_add_custom)) }
        item {
            Panel {
                SettingRow(LucideIcons.Plus, Candy.Grape, stringResource(R.string.ai_provider_add_custom), { creating = true }, first = true)
            }
        }
        item {
            Spacer(Modifier.height(18.dp))
            Panel {
                SettingRow(LucideIcons.Trash2, Candy.Tomato, stringResource(R.string.ai_delete_data), { confirmWipe = true }, first = true)
            }
        }
        item { Spacer(Modifier.windowInsetsBottomHeight(WindowInsets.navigationBars)) }
    }
    if (confirmWipe) {
        GlassDialog(
            onDismiss = { confirmWipe = false }, title = stringResource(R.string.ai_delete_data), icon = LucideIcons.Trash2, accent = Candy.Tomato,
            confirm = stringResource(R.string.ai_delete_data),
            onConfirm = {
                store.wipeAll()
                vm.aiSession.wipePersisted()
                confirmWipe = false
            },
            dismiss = stringResource(R.string.common_cancel),
        ) { Text(stringResource(R.string.ai_delete_data_body), style = ClearType.Body, color = c.inkSoft) }
    }
    editing?.let { p ->
        AiProviderDialog(vm, p, isNew = false) { editing = null }
    }
    if (creating) {
        AiProviderDialog(vm, draft, isNew = true) { creating = false }
    }
}

/** Formulir satu provider: nama, base URL, model, API key tersamar, uji koneksi, dan hapus. */
@Composable
private fun AiProviderDialog(vm: AppViewModel, initial: AiProvider, isNew: Boolean, onClose: () -> Unit) {
    val c = clear
    val store = vm.aiStore
    val uri = LocalUriHandler.current
    val scope = rememberCoroutineScope()
    var name by remember(initial.id) { mutableStateOf(initial.name) }
    var baseUrl by remember(initial.id) { mutableStateOf(initial.baseUrl) }
    var model by remember(initial.id) { mutableStateOf(initial.model) }
    var keyField by remember(initial.id) { mutableStateOf("") }
    var testState by remember(initial.id) { mutableStateOf<String?>(null) }
    var testing by remember(initial.id) { mutableStateOf(false) }
    var saveError by remember(initial.id) { mutableStateOf<String?>(null) }
    var keysVersion by remember(initial.id) { mutableStateOf(0) }
    val keyCountNow = remember(initial.id, keysVersion) { store.keyCount(initial.id) }
    var capsState by remember(initial.id) { mutableStateOf<Set<String>?>(null) }
    var capsMsg by remember(initial.id) { mutableStateOf<String?>(null) }
    var capsTesting by remember(initial.id) { mutableStateOf(false) }
    val storedCaps = remember(initial.id, initial.model) { store.caps(initial.id, initial.model) }
    // Label lencana di konteks composable (tidak boleh dari dalam lambda joinToString).
    val capTeksD = stringResource(R.string.ai_cap_teks)
    val capGambarD = stringResource(R.string.ai_cap_gambar)
    val capBerpikirD = stringResource(R.string.ai_cap_berpikir)
    val capToolD = stringResource(R.string.ai_cap_tool)
    val fallbackName = stringResource(R.string.ai_provider_custom_name)
    val errBadUrl = stringResource(R.string.ai_base_url_invalid)
    val errKeyFail = stringResource(R.string.ai_key_save_failed)

    fun current(): AiProvider = initial.copy(
        name = name.trim().ifBlank { fallbackName },
        baseUrl = baseUrl.trim(),
        model = model.trim(),
    )

    GlassDialog(
        onDismiss = onClose,
        title = if (isNew) stringResource(R.string.ai_provider_add_custom) else stringResource(R.string.ai_provider_edit),
        icon = LucideIcons.Globe, accent = Candy.Sky,
        confirm = stringResource(R.string.common_save),
        onConfirm = {
            val raw = current()
            val normalized = AiClient.validateBaseUrl(raw.baseUrl)
            when {
                normalized == null -> saveError = errBadUrl
                keyField.isNotBlank() && !(if (isNew || store.keyCount(raw.id) == 0) store.setKey(raw.id, keyField.trim()) else store.addKey(raw.id, keyField.trim())) -> saveError = errKeyFail
                else -> {
                    store.upsert(raw.copy(baseUrl = normalized))
                    onClose()
                }
            }
        },
        dismiss = stringResource(R.string.common_cancel),
    ) {
        GlassTextField(name, { name = it }, stringResource(R.string.ai_provider_name))
        Spacer(Modifier.height(10.dp))
        GlassTextField(baseUrl, { baseUrl = it }, stringResource(R.string.ai_provider_base_url))
        Spacer(Modifier.height(10.dp))
        GlassTextField(model, { model = it }, stringResource(R.string.ai_provider_model))
        Spacer(Modifier.height(10.dp))
        GlassTextField(
            keyField, { keyField = it },
            if (keyCountNow > 0) stringResource(R.string.ai_key_add_hint) else stringResource(R.string.ai_provider_api_key),
            password = true,
        )
        if (!isNew && keyCountNow > 0) {
            Spacer(Modifier.height(10.dp))
            for (i in 0 until keyCountNow) {
                Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                    Text(stringResource(R.string.ai_key_saved_n, i + 1), style = ClearType.Caption, color = c.inkSoft, modifier = Modifier.weight(1f))
                    GlassPill(onClick = { store.removeKeyAt(initial.id, i); keysVersion++ }, height = 32.dp, label = stringResource(R.string.ai_key_delete)) {
                        Text(stringResource(R.string.ai_key_delete), style = ClearType.CaptionStrong, color = Candy.Tomato)
                    }
                }
                Spacer(Modifier.height(6.dp))
            }
        }
        saveError?.let {
            Spacer(Modifier.height(8.dp))
            Text(it, style = ClearType.Caption, color = Candy.Tomato)
        }
        Spacer(Modifier.height(14.dp))
        Row(Modifier.fillMaxWidth()) {
            GlassPill(
                onClick = {
                    when {
                        !store.enabled -> testState = "needs_enabled"
                        else -> {
                            val raw = current()
                            val normalized = AiClient.validateBaseUrl(raw.baseUrl)
                            if (normalized == null) {
                                testState = "bad_url"
                            } else {
                                testing = true
                                testState = null
                                // Key tersimpan hanya dikirim ke base URL yang sama dengan yang
                                // tersimpan; URL hasil editan yang belum disimpan tidak boleh
                                // menerima key milik URL lama.
                                val key = keyField.trim().takeIf { it.isNotBlank() }
                                    ?: if (baseUrl.trim().trimEnd('/') == initial.baseUrl.trimEnd('/')) store.key(initial.id) else null
                                scope.launch {
                                    val err = AiClient().testConnection(raw.copy(baseUrl = normalized), key)
                                    testing = false
                                    testState = if (err == null) "ok" else err
                                }
                            }
                        }
                    }
                },
                height = 40.dp, label = stringResource(R.string.ai_provider_test),
            ) { Text(stringResource(R.string.ai_provider_test), style = ClearType.CaptionStrong, color = c.ink) }
            if (initial.keyUrl != null) {
                Spacer(Modifier.width(8.dp))
                GlassPill(onClick = { uri.openUri(initial.keyUrl) }, height = 40.dp, label = stringResource(R.string.ai_provider_get_key)) {
                    Text(stringResource(R.string.ai_provider_get_key), style = ClearType.CaptionStrong, color = c.ink)
                }
            }
        }
        when {
            testing -> Text(stringResource(R.string.ai_provider_testing), style = ClearType.Caption, color = c.inkSoft, modifier = Modifier.padding(top = 10.dp))
            testState == "ok" -> Text(stringResource(R.string.ai_provider_test_ok), style = ClearType.Caption, color = Candy.Mint, modifier = Modifier.padding(top = 10.dp))
            testState == "needs_enabled" -> Text(stringResource(R.string.ai_test_needs_enabled), style = ClearType.Caption, color = Candy.Tomato, modifier = Modifier.padding(top = 10.dp))
            testState == "bad_url" -> Text(stringResource(R.string.ai_base_url_invalid), style = ClearType.Caption, color = Candy.Tomato, modifier = Modifier.padding(top = 10.dp))
            testState != null -> Text(stringResource(R.string.ai_provider_test_fail, testState!!), style = ClearType.Caption, color = Candy.Tomato, modifier = Modifier.padding(top = 10.dp))
        }
        Spacer(Modifier.height(14.dp))
        Row(Modifier.fillMaxWidth()) {
            GlassPill(
                onClick = {
                    when {
                        !store.enabled -> capsMsg = "needs_enabled"
                        else -> {
                            val raw = current()
                            val normalized = AiClient.validateBaseUrl(raw.baseUrl)
                            if (normalized == null) {
                                capsMsg = "bad_url"
                            } else {
                                capsTesting = true
                                capsMsg = null
                                val key = keyField.trim().takeIf { it.isNotBlank() }
                                    ?: if (baseUrl.trim().trimEnd('/') == initial.baseUrl.trimEnd('/')) store.key(initial.id) else null
                                scope.launch {
                                    val caps = AiClient().probeCapabilities(raw.copy(baseUrl = normalized), key)
                                    capsTesting = false
                                    if (caps.isEmpty()) {
                                        capsMsg = "fail"
                                    } else {
                                        store.setCaps(raw.id, raw.model, caps)
                                        capsState = caps
                                        capsMsg = "ok"
                                    }
                                }
                            }
                        }
                    }
                },
                height = 40.dp, label = stringResource(R.string.ai_caps_test),
            ) { Text(stringResource(R.string.ai_caps_test), style = ClearType.CaptionStrong, color = c.ink) }
        }
        val shownCaps = capsState ?: storedCaps
        when {
            capsTesting -> Text(stringResource(R.string.ai_caps_testing), style = ClearType.Caption, color = c.inkSoft, modifier = Modifier.padding(top = 10.dp))
            capsMsg == "needs_enabled" -> Text(stringResource(R.string.ai_test_needs_enabled), style = ClearType.Caption, color = Candy.Tomato, modifier = Modifier.padding(top = 10.dp))
            capsMsg == "bad_url" -> Text(stringResource(R.string.ai_base_url_invalid), style = ClearType.Caption, color = Candy.Tomato, modifier = Modifier.padding(top = 10.dp))
            capsMsg == "fail" -> Text(stringResource(R.string.ai_caps_none), style = ClearType.Caption, color = Candy.Tomato, modifier = Modifier.padding(top = 10.dp))
            shownCaps.isNotEmpty() -> Text(
                stringResource(
                    R.string.ai_caps_result,
                    shownCaps.joinToString(" · ") { cap ->
                        when (cap) {
                            "teks" -> capTeksD
                            "gambar" -> capGambarD
                            "berpikir" -> capBerpikirD
                            "tool" -> capToolD
                            else -> cap
                        }
                    },
                ),
                style = ClearType.Caption, color = Candy.Mint, modifier = Modifier.padding(top = 10.dp),
            )
        }
        if (!isNew) {
            Spacer(Modifier.height(14.dp))
            Row(Modifier.fillMaxWidth()) {
                if (store.active?.id != initial.id) {
                    GlassPill(onClick = { store.setActive(initial.id); onClose() }, height = 40.dp, label = stringResource(R.string.ai_provider_make_active)) {
                        Text(stringResource(R.string.ai_provider_make_active), style = ClearType.CaptionStrong, color = c.ink)
                    }
                    Spacer(Modifier.width(8.dp))
                }
                GlassPill(onClick = { store.remove(initial.id); onClose() }, height = 40.dp, label = stringResource(R.string.ai_provider_delete)) {
                    Text(stringResource(R.string.ai_provider_delete), style = ClearType.CaptionStrong, color = Candy.Tomato)
                }
            }
        }
    }
}
