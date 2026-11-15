package com.ledocs.app.ai

import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.StartOffset
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsTopHeight
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import com.ledocs.app.R
import com.ledocs.app.engine.DocumentTypes
import com.ledocs.app.ui.AppViewModel
import com.ledocs.app.ui.LucideIcons
import com.ledocs.app.ui.desk.ScreenHeader
import com.ledocs.app.ui.glass.Capsule
import com.ledocs.app.ui.glass.GlassCircle
import com.ledocs.app.ui.glass.GlassDialog
import com.ledocs.app.ui.glass.GlassIcon
import com.ledocs.app.ui.glass.GlassPill
import com.ledocs.app.ui.glass.GlassTab
import com.ledocs.app.ui.glass.GlassTabs
import com.ledocs.app.ui.glass.GlassTextField
import com.ledocs.app.ui.glass.RoundedRectangle
import com.ledocs.app.ui.glass.WashPanel
import com.ledocs.app.ui.glass.glass
import com.ledocs.app.ui.glass.pressable
import com.ledocs.app.ui.glass.wash
import com.ledocs.app.ui.home.FileRow
import com.ledocs.app.ui.home.dayBucket
import com.ledocs.app.ui.home.displayName
import com.ledocs.app.ui.home.whenLabel
import com.ledocs.app.ui.theme.Candy
import com.ledocs.app.ui.theme.ClearType
import com.ledocs.app.ui.theme.clear
import com.ledocs.app.ui.viewer.formatSize
import java.io.File
import java.net.URL
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/** Apakah perangkat sedang punya jalur internet (bukan sekadar tersambung ke jaringan). */
private fun onlineNow(context: android.content.Context): Boolean = runCatching {
    val cm = context.getSystemService(ConnectivityManager::class.java) ?: return false
    val caps = cm.getNetworkCapabilities(cm.activeNetwork) ?: return false
    caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
}.getOrDefault(false)

/**
 * Layar Asisten AI: teks bersih tanpa ikon hias, jawaban dirender Markdown, penalaran model
 * tampil sebagai blok Berpikir yang bisa dilipat, dan latar aurora monokrom khusus layar ini.
 * Dokumen yang sedang terbuka menempel sendiri sebagai konteks; dokumen lain bisa ditambah.
 * Sesi hidup di AppViewModel, jadi menutup layar tidak membunuh stream yang sedang berjalan.
 */
@Composable
fun AiScreen(vm: AppViewModel) {
    val c = clear
    val context = LocalContext.current
    val store = vm.aiStore
    val session = vm.aiSession
    val doc = vm.document
    var page by remember { mutableStateOf("chat") }
    var pickerMode by remember { mutableStateOf("context") }
    BackHandler(enabled = page == "chat") { vm.aiOpen = false }
    BackHandler(enabled = page == "history") { page = "chat" }
    BackHandler(enabled = page == "pick") { page = "chat" }
    LaunchedEffect(doc?.id) {
        if (doc != null) session.attach(doc)
        else if (session.attachedDocId != null) session.detach()
    }

    var online by remember { mutableStateOf(onlineNow(context)) }
    DisposableEffect(context) {
        val cm = context.getSystemService(ConnectivityManager::class.java)
        val cb = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) { online = onlineNow(context) }
            override fun onLost(network: Network) { online = onlineNow(context) }
            override fun onCapabilitiesChanged(network: Network, caps: NetworkCapabilities) { online = onlineNow(context) }
        }
        runCatching { cm?.registerDefaultNetworkCallback(cb) }
        onDispose { runCatching { cm?.unregisterNetworkCallback(cb) } }
    }

    var input by remember { mutableStateOf("") }
    var picker by remember { mutableStateOf(false) }
    var pendingAction by remember { mutableStateOf<(() -> Unit)?>(null) }
    var saveTarget by remember { mutableStateOf<AiEntry?>(null) }
    var refSheet by remember { mutableStateOf(false) }
    var noVisionWarn by remember { mutableStateOf(false) }
    var addingCommand by remember { mutableStateOf(false) }
    val uiScope = androidx.compose.runtime.rememberCoroutineScope()

    // Pemilih gambar referensi: salin ke folder refs sesi, lalu daftarkan sebagai pedoman gambar.
    val imagePicker = androidx.activity.compose.rememberLauncherForActivityResult(
        androidx.activity.result.contract.ActivityResultContracts.PickMultipleVisualMedia(4),
    ) { uris ->
        if (uris.isEmpty()) return@rememberLauncherForActivityResult
        uiScope.launch {
            for (uri in uris) {
                val copied = withContext(Dispatchers.IO) {
                    runCatching {
                        val dir = File(context.filesDir, "ai_refs").apply { mkdirs() }
                        val name = runCatching {
                            context.contentResolver.query(uri, arrayOf(android.provider.OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cur ->
                                if (cur.moveToFirst()) cur.getString(0) else null
                            }
                        }.getOrNull() ?: (uri.lastPathSegment?.substringAfterLast('/') ?: "gambar")
                        val target = File(dir, System.currentTimeMillis().toString(36) + "-" + name.replace(Regex("[^A-Za-z0-9._-]"), "_"))
                        try {
                            context.contentResolver.openInputStream(uri)?.use { input -> target.outputStream().use { input.copyTo(it) } }
                        } catch (e: Exception) {
                            target.delete()
                            throw e
                        }
                        if (target.exists() && target.length() > 0) target to name else { target.delete(); null }
                    }.getOrNull()
                }
                if (copied != null) session.addReferenceImageFile(copied.first, copied.second)
            }
        }
    }
    val provider = store.active
    // Provider lokal (server di perangkat sendiri) tidak butuh internet sama sekali.
    val reachable = online || (provider != null && AiClient.isLoopback(provider.baseUrl))
    LaunchedEffect(provider?.id) { if (provider == null) pendingAction = null }

    fun hostOf(p: AiProvider): String = runCatching { URL(p.baseUrl).host }.getOrDefault(p.baseUrl)

    /** Gerbang persetujuan: kiriman pertama yang membawa teks dokumen ke provider ini wajib izin. */
    fun withConsent(action: () -> Unit) {
        val p = provider ?: return
        val docAttached = session.context is AiContext.Attached || session.preparing ||
            session.extraDocs.any { it.text != null || it.loading } ||
            session.references.any { it.text != null || it.isImage || it.loading }
        if (docAttached && !AiClient.isLoopback(p.baseUrl) && !store.consented(p.id, hostOf(p), session.attachedDocPath)) {
            pendingAction = action
        } else {
            action()
        }
    }

    fun sendText(text: String) {
        val clean = text.trim()
        if (clean.isEmpty() || session.running || session.preparing) return
        if (!reachable) {
            session.enqueue(clean)
        } else {
            withConsent { session.send(clean) }
        }
    }

    fun trySend() {
        val text = input
        if (text.isBlank() || session.running || session.preparing) return
        input = ""
        sendText(text)
    }

    // Antrean offline terkirim sendiri begitu jaringan kembali (lewat gerbang persetujuan).
    LaunchedEffect(reachable) {
        if (reachable && provider != null) withConsent { session.flushQueue() }
    }

    Box(Modifier.fillMaxSize()) {
        AiAuroraBackground(Modifier.fillMaxSize())
        if (page == "history") {
            AiHistoryPage(vm, onBack = { page = "chat" })
        } else if (page == "pick") {
            AiPickerPage(
                vm = vm,
                forReference = pickerMode == "reference",
                excludeKeys = buildSet {
                    session.attachedDocPath?.let { add(it) }
                    if (pickerMode == "reference") session.references.forEach { add(it.uriString) }
                    else session.extraDocs.forEach { add(it.uriString) }
                },
                onBack = { page = "chat" },
                onConfirm = { rows ->
                    rows.forEach { row ->
                        if (pickerMode == "reference") session.addReferenceDoc(row.key, row.name)
                        else session.addExtraDoc(row.key, row.name)
                    }
                    page = "chat"
                },
            )
        } else {
            Column(Modifier.fillMaxSize()) {
                Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars))
                Spacer(Modifier.height(14.dp))
                ScreenHeader(null, stringResource(R.string.ai_title), onBack = { vm.aiOpen = false }) {
                    GlassCircle(onClick = { page = "history" }, size = 40.dp, label = stringResource(R.string.ai_history)) {
                        GlassIcon(LucideIcons.RotateCcw, null, Modifier.size(19.dp))
                    }
                    Spacer(Modifier.width(8.dp))
                    GlassCircle(onClick = { session.newChat() }, size = 40.dp, label = stringResource(R.string.ai_new_chat)) {
                        GlassIcon(LucideIcons.MessageSquarePlus, null, Modifier.size(19.dp))
                    }
                }
                // pil provider: teks saja, nama dan model aktif, ketuk untuk mengganti
                Row(Modifier.fillMaxWidth().padding(horizontal = 22.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                    GlassPill(onClick = { picker = true }, height = 40.dp, horizontalPadding = 14.dp, label = provider?.name) {
                        Text(
                            if (provider == null) stringResource(R.string.ai_open_settings) else "${provider.name} · ${provider.model}",
                            style = ClearType.CaptionStrong, color = c.ink,
                        )
                    }
                }
                // chip konteks dokumen
                if (doc != null) {
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 22.dp, vertical = 2.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            when {
                                session.preparing -> stringResource(R.string.ai_context_preparing)
                                else -> when (val ctx = session.context) {
                                    is AiContext.Attached -> stringResource(R.string.ai_context_pages, ctx.name, ctx.pages) +
                                        if (ctx.truncated) " · " + stringResource(R.string.ai_context_truncated) else ""
                                    AiContext.Unavailable -> stringResource(R.string.ai_context_unavailable)
                                    else -> doc.name
                                }
                            },
                            style = ClearType.Caption, color = c.inkSoft, modifier = Modifier.weight(1f),
                        )
                        if (session.attachedDocId != null) {
                            GlassPill(onClick = { session.detach() }, height = 30.dp, horizontalPadding = 10.dp, label = stringResource(R.string.ai_detach)) {
                                Text(stringResource(R.string.ai_detach_short), style = ClearType.Caption, color = c.ink)
                            }
                        }
                    }
                }
                // chip dokumen konteks tambahan + tombol tambah
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = 22.dp, vertical = 2.dp).horizontalScroll(rememberScrollState()),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    session.extraDocs.forEach { extra ->
                        GlassPill(onClick = { session.removeExtraDoc(extra) }, height = 30.dp, horizontalPadding = 10.dp, label = extra.name) {
                            Text(
                                extra.name + when {
                                    extra.loading -> " · " + stringResource(R.string.ai_extra_loading)
                                    extra.failed -> " · " + stringResource(R.string.ai_extra_failed)
                                    else -> " · ✕"
                                },
                                style = ClearType.Caption, color = if (extra.failed) Candy.Tomato else c.ink,
                            )
                        }
                    }
                    GlassPill(onClick = { pickerMode = "context"; page = "pick" }, height = 30.dp, horizontalPadding = 10.dp, label = stringResource(R.string.ai_extra_add)) {
                        Text("+ " + stringResource(R.string.ai_extra_add), style = ClearType.Caption, color = c.ink)
                    }
                }
                // chip pedoman dan referensi (dari tombol +): peran berbeda dari dokumen konteks,
                // ini acuan yang diikuti AI, jadi barisnya sendiri dengan label Pedoman/Gambar.
                if (session.references.isNotEmpty()) {
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 22.dp, vertical = 2.dp).horizontalScroll(rememberScrollState()),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        session.references.forEach { ref ->
                            GlassPill(onClick = { session.removeReference(ref) }, height = 30.dp, horizontalPadding = 10.dp, label = ref.name) {
                                Text(
                                    stringResource(if (ref.isImage) R.string.ai_ref_chip_image else R.string.ai_ref_chip_doc) + " · " + ref.name + when {
                                        ref.loading -> " · " + stringResource(R.string.ai_extra_loading)
                                        ref.failed -> " · " + stringResource(R.string.ai_extra_failed)
                                        else -> " · ✕"
                                    },
                                    style = ClearType.Caption, color = if (ref.failed) Candy.Tomato else Candy.Lemon,
                                )
                            }
                        }
                    }
                }
                // meter konteks
                if (session.contextChars > 0) {
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 22.dp, vertical = 3.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Box(Modifier.weight(1f).height(3.dp).clip(Capsule).background(c.ink.copy(alpha = 0.15f))) {
                            Box(Modifier.fillMaxWidth(session.contextFraction).height(3.dp).clip(Capsule).background(c.ink.copy(alpha = 0.75f)))
                        }
                        Spacer(Modifier.width(10.dp))
                        Text(
                            stringResource(R.string.ai_context_meter, (session.contextFraction * 100).toInt()),
                            style = ClearType.Caption, color = c.inkSoft,
                        )
                    }
                }

                val listState = rememberLazyListState()
                LaunchedEffect(session) {
                    snapshotFlow { session.entries.size to (session.entries.lastOrNull()?.text?.length ?: 0) }
                        .collect {
                            if (session.entries.isNotEmpty() && !listState.isScrollInProgress) {
                                // Kartu ringkasan dan kartu pemahaman (bila ada) menempati indeks awal, entri sesudahnya.
                                val cards = (if (session.summary != null) 1 else 0) + (if (session.referenceUnderstanding != null) 1 else 0)
                                val target = session.entries.size - 1 + cards
                                listState.animateScrollToItem(target)
                            }
                        }
                }
                Box(Modifier.weight(1f).fillMaxWidth()) {
                    when {
                        provider == null -> Column(
                            Modifier.fillMaxSize().padding(horizontal = 30.dp),
                            verticalArrangement = Arrangement.Center,
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            Text(stringResource(R.string.ai_no_provider), style = ClearType.Body, color = c.inkSoft)
                            Spacer(Modifier.height(16.dp))
                            GlassPill(onClick = { vm.aiOpen = false; vm.settingsOpen = true }, label = stringResource(R.string.ai_open_settings)) {
                                Text(stringResource(R.string.ai_open_settings), style = ClearType.BodyStrong, color = c.ink)
                            }
                        }
                        session.entries.isEmpty() && !reachable -> Column(
                            Modifier.fillMaxSize().padding(horizontal = 30.dp),
                            verticalArrangement = Arrangement.Center,
                            horizontalAlignment = Alignment.CenterHorizontally,
                        ) {
                            Text(stringResource(R.string.ai_offline), style = ClearType.Title, color = c.ink)
                            Spacer(Modifier.height(8.dp))
                            Text(stringResource(R.string.ai_offline_body), style = ClearType.Body, color = c.inkSoft)
                        }
                        session.entries.isEmpty() -> Column(
                            Modifier.fillMaxSize().padding(horizontal = 22.dp, vertical = 10.dp),
                            verticalArrangement = Arrangement.spacedBy(10.dp),
                        ) {
                            session.summary?.let { s ->
                                SummaryCard(s)
                            }
                            session.referenceUnderstanding?.let { g ->
                                GuideCard(g)
                            }
                            Text(
                                if (doc != null) stringResource(R.string.ai_context_doc) else stringResource(R.string.ai_attach_hint),
                                style = ClearType.Body, color = c.inkSoft,
                            )
                            val suggestions = if (doc != null) {
                                listOf(stringResource(R.string.ai_suggest_summarize), stringResource(R.string.ai_suggest_main_points))
                            } else {
                                listOf(stringResource(R.string.ai_suggest_letter), stringResource(R.string.ai_suggest_explain))
                            }
                            suggestions.forEach { s ->
                                GlassPill(onClick = { input = s }, height = 44.dp, label = s) {
                                    Text(s, style = ClearType.BodyStrong, color = c.ink)
                                }
                            }
                        }
                        else -> LazyColumn(
                            state = listState,
                            modifier = Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(horizontal = 22.dp, vertical = 10.dp),
                            verticalArrangement = Arrangement.spacedBy(14.dp),
                        ) {
                            session.summary?.let { s ->
                                item(key = "summary") { SummaryCard(s) }
                            }
                            session.referenceUnderstanding?.let { g ->
                                item(key = "guide") { GuideCard(g) }
                            }
                            items(session.entries) { entry ->
                                EntryRow(
                                    entry = entry,
                                    isLast = entry === session.entries.lastOrNull(),
                                    isRunningEntry = session.running && entry === session.entries.lastOrNull() && entry.role == "assistant",
                                    onRetry = { withConsent { session.retry(entry) } },
                                    onContinue = { withConsent { session.continueAnswer(entry) } },
                                    onSave = { saveTarget = entry },
                                    onOpenSaved = { path -> vm.open(Uri.fromFile(File(path))) },
                                )
                            }
                            if (session.followUps.isNotEmpty() && !session.running) {
                                item(key = "followups") {
                                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                        session.followUps.forEach { q ->
                                            GlassPill(onClick = { sendText(q) }, height = 40.dp, label = q) {
                                                Text(q, style = ClearType.CaptionStrong, color = c.ink)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // bar gaya + perintah cepat
                if (provider != null) {
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp).horizontalScroll(rememberScrollState()),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        AiStyle.entries.forEach { style ->
                            val selected = session.style == style
                            GlassPill(
                                onClick = { session.setStyle(style) }, height = 34.dp, horizontalPadding = 12.dp,
                                surface = if (selected) Candy.Sky else androidx.compose.ui.graphics.Color.Unspecified,
                                label = style.name,
                            ) {
                                Text(
                                    stringResource(
                                        when (style) {
                                            AiStyle.Ringkas -> R.string.ai_style_ringkas
                                            AiStyle.Seimbang -> R.string.ai_style_seimbang
                                            AiStyle.Kreatif -> R.string.ai_style_kreatif
                                        },
                                    ),
                                    style = ClearType.CaptionStrong,
                                    color = if (selected) androidx.compose.ui.graphics.Color.White else c.ink,
                                )
                            }
                        }
                        store.commands.forEach { cmd ->
                            GlassPill(onClick = { sendText(cmd.text) }, height = 34.dp, horizontalPadding = 12.dp, label = cmd.label) {
                                Text(cmd.label, style = ClearType.CaptionStrong, color = c.ink)
                            }
                        }
                        GlassPill(onClick = { addingCommand = true }, height = 34.dp, horizontalPadding = 12.dp, label = stringResource(R.string.ai_commands_add)) {
                            Text("+ " + stringResource(R.string.ai_commands_add), style = ClearType.CaptionStrong, color = c.inkSoft)
                        }
                    }
                    if (!reachable) {
                        Text(
                            stringResource(R.string.ai_queue_hint),
                            style = ClearType.Caption, color = c.inkSoft,
                            modifier = Modifier.padding(horizontal = 22.dp, vertical = 2.dp),
                        )
                    }
                    // bar ketik: kapsul kaca yang sama bahasanya dengan dock, duduk di posisi dock
                    Row(
                        Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = 16.dp, vertical = 12.dp)
                            .glass(Capsule, c.glass, blur = 3.dp, lensHeight = 14.dp, lensAmount = 26.dp)
                            .drawBehind { drawRect(c.capsule, alpha = 0.9f) }
                            .padding(start = 6.dp, end = 6.dp, top = 6.dp, bottom = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        // Tombol + di pojok kiri: unggah pedoman atau referensi yang diikuti AI
                        // (beda peran dari chip Tambah dokumen konteks di atas).
                        GlassCircle(onClick = { refSheet = true }, size = 40.dp, label = stringResource(R.string.ai_ref_button)) {
                            GlassIcon(LucideIcons.Plus, null, Modifier.size(20.dp))
                        }
                        Spacer(Modifier.width(10.dp))
                        BasicTextField(
                            value = input,
                            onValueChange = { input = it },
                            modifier = Modifier.weight(1f),
                            textStyle = ClearType.Body.copy(color = c.ink),
                            cursorBrush = SolidColor(c.ink),
                            maxLines = 4,
                            decorationBox = { inner ->
                                Box {
                                    if (input.isEmpty()) Text(stringResource(R.string.ai_input_hint), style = ClearType.Body, color = c.inkSoft)
                                    inner()
                                }
                            },
                        )
                        Spacer(Modifier.width(8.dp))
                        if (session.running) {
                            Box(
                                Modifier.size(46.dp).pressable({ session.stop() }, pressedScale = 0.9f, feel = null).clip(CircleShape).background(androidx.compose.ui.graphics.Color.White),
                                contentAlignment = Alignment.Center,
                            ) { Box(Modifier.size(16.dp).clip(RoundedRectangle(4.dp)).background(Candy.Tomato)) }
                        } else {
                            GlassCircle(
                                onClick = { trySend() }, size = 46.dp,
                                surface = if (input.isBlank() || session.preparing) c.glass else Candy.Sky,
                                label = stringResource(R.string.ai_send),
                            ) { GlassIcon(LucideIcons.Send, null, Modifier.size(20.dp), tint = if (input.isBlank() || session.preparing) c.inkSoft else androidx.compose.ui.graphics.Color.White) }
                        }
                    }
                }
            }
        }
    }

    if (picker) {
        GlassDialog(
            onDismiss = { picker = false }, title = stringResource(R.string.ai_providers_title),
            dismiss = stringResource(R.string.common_cancel),
        ) {
            store.providers.forEach { p ->
                Row(
                    Modifier.fillMaxWidth().pressable({ store.setActive(p.id); picker = false }).padding(vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(Modifier.weight(1f)) {
                        Text(p.name, style = ClearType.BodyStrong, color = c.ink)
                        Text(p.model, style = ClearType.Caption, color = c.inkSoft)
                        val caps = store.caps(p.id, p.model)
                        if (caps.isNotEmpty()) {
                            Text(
                                stringResource(R.string.ai_caps_result, capsLabel(caps)),
                                style = ClearType.Caption, color = c.inkSoft,
                            )
                        }
                    }
                    if (store.active?.id == p.id) GlassIcon(LucideIcons.Check, null, Modifier.size(18.dp), tint = Candy.Mint)
                }
            }
            Spacer(Modifier.height(6.dp))
            GlassPill(onClick = { picker = false; vm.aiOpen = false; vm.settingsOpen = true }, height = 42.dp, label = stringResource(R.string.ai_providers_row)) {
                Text(stringResource(R.string.ai_providers_row), style = ClearType.BodyStrong, color = c.ink)
            }
        }
    }

    saveTarget?.let { entry ->
        GlassDialog(
            onDismiss = { saveTarget = null }, title = stringResource(R.string.ai_save_as),
            dismiss = stringResource(R.string.common_cancel),
        ) {
            Row {
                GlassPill(onClick = { session.saveAnswerAs(entry, "pdf"); saveTarget = null }, height = 42.dp, label = stringResource(R.string.ai_save_pdf)) {
                    Text(stringResource(R.string.ai_save_pdf), style = ClearType.BodyStrong, color = c.ink)
                }
                Spacer(Modifier.width(8.dp))
                GlassPill(onClick = { session.saveAnswerAs(entry, "docx"); saveTarget = null }, height = 42.dp, label = stringResource(R.string.ai_save_docx)) {
                    Text(stringResource(R.string.ai_save_docx), style = ClearType.BodyStrong, color = c.ink)
                }
            }
        }
    }

    if (refSheet) {
        GlassDialog(
            onDismiss = { refSheet = false }, title = stringResource(R.string.ai_ref_title),
            dismiss = stringResource(R.string.common_cancel),
        ) {
            Row(
                Modifier.fillMaxWidth().pressable({ refSheet = false; pickerMode = "reference"; page = "pick" }).padding(vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                GlassIcon(LucideIcons.FileText, null, Modifier.size(22.dp), tint = Candy.Lemon)
                Spacer(Modifier.width(12.dp))
                Column(Modifier.weight(1f)) {
                    Text(stringResource(R.string.ai_ref_doc), style = ClearType.BodyStrong, color = c.ink)
                    Text(stringResource(R.string.ai_ref_doc_body), style = ClearType.Caption, color = c.inkSoft)
                }
            }
            Row(
                Modifier.fillMaxWidth().pressable({
                    refSheet = false
                    // Jujur di muka: model yang sudah teruji TANPA kemampuan gambar tidak
                    // diberi gambar sama sekali; yang belum teruji dibiarkan mencoba.
                    val p = provider
                    val caps = if (p != null) store.caps(p.id, p.model) else emptySet()
                    if (p != null && caps.isNotEmpty() && "gambar" !in caps) {
                        noVisionWarn = true
                    } else {
                        imagePicker.launch(
                            androidx.activity.result.PickVisualMediaRequest(
                                androidx.activity.result.contract.ActivityResultContracts.PickVisualMedia.ImageOnly,
                            ),
                        )
                    }
                }).padding(vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                GlassIcon(LucideIcons.Images, null, Modifier.size(22.dp), tint = Candy.Lemon)
                Spacer(Modifier.width(12.dp))
                Column(Modifier.weight(1f)) {
                    Text(stringResource(R.string.ai_ref_image), style = ClearType.BodyStrong, color = c.ink)
                    Text(stringResource(R.string.ai_ref_image_body), style = ClearType.Caption, color = c.inkSoft)
                }
            }
        }
    }

    if (noVisionWarn) {
        GlassDialog(
            onDismiss = { noVisionWarn = false }, title = stringResource(R.string.ai_ref_image),
            dismiss = stringResource(R.string.common_ok),
        ) {
            Text(stringResource(R.string.ai_ref_no_vision), style = ClearType.Body, color = c.inkSoft)
        }
    }

    if (addingCommand) {
        var label by remember { mutableStateOf("") }
        var body by remember { mutableStateOf("") }
        GlassDialog(
            onDismiss = { addingCommand = false }, title = stringResource(R.string.ai_commands_add),
            confirm = stringResource(R.string.common_save),
            onConfirm = {
                if (label.isNotBlank() && body.isNotBlank()) store.addCommand(label.trim(), body.trim())
                addingCommand = false
            },
            dismiss = stringResource(R.string.common_cancel),
        ) {
            GlassTextField(label, { label = it }, stringResource(R.string.ai_command_label))
            Spacer(Modifier.height(10.dp))
            GlassTextField(body, { body = it }, stringResource(R.string.ai_command_text))
        }
    }

    pendingAction?.let { action ->
        val p = provider
        if (p != null) {
            val host = hostOf(p)
            GlassDialog(
                onDismiss = { pendingAction = null }, title = stringResource(R.string.ai_consent_title),
                confirm = stringResource(R.string.ai_consent_send),
                onConfirm = {
                    store.consent(p.id, host, session.attachedDocPath)
                    pendingAction = null
                    action()
                },
                dismiss = stringResource(R.string.common_cancel),
            ) {
                Text(stringResource(R.string.ai_consent_body, host, p.model), style = ClearType.Body, color = c.inkSoft)
                (session.context as? AiContext.Attached)?.let { ctx ->
                    Spacer(Modifier.height(8.dp))
                    Text(
                        stringResource(R.string.ai_consent_scope, ctx.name, ctx.pages, session.contextChars),
                        style = ClearType.Caption, color = c.inkSoft,
                    )
                }
            }
        }
    }
}

/** Label kemampuan yang bisa dibaca pengguna dari himpunan label internal hasil uji. */
@Composable
private fun capsLabel(caps: Set<String>): String {
    val parts = mutableListOf<String>()
    if ("teks" in caps) parts += stringResource(R.string.ai_cap_teks)
    if ("gambar" in caps) parts += stringResource(R.string.ai_cap_gambar)
    if ("berpikir" in caps) parts += stringResource(R.string.ai_cap_berpikir)
    if ("tool" in caps) parts += stringResource(R.string.ai_cap_tool)
    return parts.joinToString(" · ")
}

/** Kartu pemahaman pedoman: yang AI tangkap dari referensi, untuk dikoreksi pengguna lewat pesan. */
@Composable
private fun GuideCard(text: String) {
    val c = clear
    Column(
        Modifier.fillMaxWidth().wash(RoundedRectangle(18.dp), c.capsule).padding(horizontal = 16.dp, vertical = 12.dp),
    ) {
        Text(stringResource(R.string.ai_guide_title), style = ClearType.CaptionStrong, color = Candy.Lemon)
        Spacer(Modifier.height(6.dp))
        MarkdownText(text)
        Spacer(Modifier.height(6.dp))
        Text(stringResource(R.string.ai_guide_hint), style = ClearType.Caption, color = c.inkSoft)
    }
}

/** Kartu ringkasan otomatis dokumen yang menempel. */
@Composable
private fun SummaryCard(text: String) {
    val c = clear
    Column(
        Modifier.fillMaxWidth().wash(RoundedRectangle(18.dp), c.capsule).padding(horizontal = 16.dp, vertical = 12.dp),
    ) {
        Text(stringResource(R.string.ai_summary_title), style = ClearType.CaptionStrong, color = c.ink)
        Spacer(Modifier.height(6.dp))
        MarkdownText(text)
        Spacer(Modifier.height(6.dp))
        Text(stringResource(R.string.ai_summary_auto), style = ClearType.Caption, color = c.inkSoft)
    }
}

/** Tiga titik beranimasi gelombang: indikator jawaban sedang disiapkan atau masih mengalir. */
@Composable
private fun TypingDots() {
    val c = clear
    val transition = rememberInfiniteTransition(label = "ai-dots")
    Row(horizontalArrangement = Arrangement.spacedBy(5.dp), modifier = Modifier.padding(vertical = 8.dp)) {
        for (i in 0..2) {
            val alpha by transition.animateFloat(
                initialValue = 0.25f, targetValue = 1f,
                animationSpec = infiniteRepeatable(tween(520), RepeatMode.Reverse, initialStartOffset = StartOffset(i * 170)),
                label = "dot$i",
            )
            Box(Modifier.size(7.dp).clip(CircleShape).background(c.ink.copy(alpha = alpha)))
        }
    }
}

@Composable
private fun EntryRow(
    entry: AiEntry,
    isLast: Boolean,
    isRunningEntry: Boolean,
    onRetry: () -> Unit,
    onContinue: () -> Unit,
    onSave: () -> Unit,
    onOpenSaved: (String) -> Unit,
) {
    val c = clear
    if (entry.role == "user") {
        Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.End) {
            // Gelembung mengikuti panjang teksnya (kata pendek jadi gelembung kecil), dibatasi
            // maksimal 85 persen lebar daftar; bukan fillMaxWidth yang memaksa lebar penuh.
            BoxWithConstraints {
                Box(
                    // Gelembung di daftar yang bergeser memakai wash (pola codebase untuk permukaan
                    // yang scroll), bukan backdrop blur asli yang mahal per frame.
                    Modifier.widthIn(max = maxWidth * 0.85f).wash(Capsule, c.capsule)
                        .padding(horizontal = 16.dp, vertical = 10.dp),
                ) { SelectionContainer { Text(entry.text, style = ClearType.Body, color = c.ink) } }
            }
            if (entry.queued) {
                Spacer(Modifier.height(4.dp))
                Text(stringResource(R.string.ai_queued), style = ClearType.Caption, color = c.inkSoft)
            }
        }
        return
    }
    Column(Modifier.fillMaxWidth()) {
        if (entry.reasoning.isNotBlank()) {
            var open by remember(entry) { mutableStateOf<Boolean?>(null) }
            val expanded = open ?: entry.text.isEmpty()
            Column(
                Modifier.fillMaxWidth().wash(RoundedRectangle(14.dp), c.capsule).padding(horizontal = 14.dp, vertical = 10.dp),
            ) {
                Row(
                    Modifier.fillMaxWidth().pressable({ open = !expanded }),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        if (entry.text.isEmpty()) stringResource(R.string.ai_thinking_label) else stringResource(R.string.ai_thought_done),
                        style = ClearType.CaptionStrong, color = c.ink,
                    )
                }
                if (expanded) {
                    Spacer(Modifier.height(6.dp))
                    Text(entry.reasoning, style = ClearType.Caption, color = c.inkSoft)
                }
            }
            Spacer(Modifier.height(8.dp))
        }
        if (entry.text.isNotEmpty()) {
            SelectionContainer { MarkdownText(entry.text) }
        } else if (isRunningEntry && entry.reasoning.isEmpty()) {
            TypingDots()
        }
        if (isRunningEntry && entry.text.isNotEmpty()) TypingDots()
        if (entry.stopped) Text(stringResource(R.string.ai_stopped), style = ClearType.Caption, color = c.inkSoft)
        if (entry.interrupted) Text(stringResource(R.string.ai_interrupted), style = ClearType.Caption, color = Candy.Tangerine)
        if (entry.failed) {
            entry.error?.let { Text(it, style = ClearType.Body, color = Candy.Tomato) }
        }
        if (entry.saving) {
            Spacer(Modifier.height(6.dp))
            Text(stringResource(R.string.ai_saving), style = ClearType.Caption, color = c.inkSoft)
        }
        if (entry.saveFailed) {
            Spacer(Modifier.height(6.dp))
            Text(stringResource(R.string.ai_save_failed), style = ClearType.Caption, color = Candy.Tomato)
        }
        entry.savedName?.let { name ->
            Spacer(Modifier.height(6.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.ai_saved_as, name), style = ClearType.Caption, color = c.inkSoft, modifier = Modifier.weight(1f))
                entry.savedPath?.let { path ->
                    GlassPill(onClick = { onOpenSaved(path) }, height = 32.dp, horizontalPadding = 12.dp, label = stringResource(R.string.ai_open_saved)) {
                        Text(stringResource(R.string.ai_open_saved), style = ClearType.CaptionStrong, color = c.ink)
                    }
                }
            }
        }
        val showActions = !isRunningEntry && (entry.text.isNotBlank() || entry.failed)
        if (showActions) {
            Spacer(Modifier.height(8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                val clipboard = LocalClipboardManager.current
                if (entry.text.isNotBlank()) {
                    GlassPill(onClick = { clipboard.setText(AnnotatedString(entry.text)) }, height = 34.dp, horizontalPadding = 12.dp, label = stringResource(R.string.ai_copy)) {
                        Text(stringResource(R.string.ai_copy), style = ClearType.CaptionStrong, color = c.ink)
                    }
                    GlassPill(onClick = onSave, height = 34.dp, horizontalPadding = 12.dp, label = stringResource(R.string.ai_save_as)) {
                        Text(stringResource(R.string.ai_save_as), style = ClearType.CaptionStrong, color = c.ink)
                    }
                }
                if (entry.canContinue && isLast) {
                    GlassPill(onClick = onContinue, height = 34.dp, horizontalPadding = 12.dp, label = stringResource(R.string.ai_continue)) {
                        Text(stringResource(R.string.ai_continue), style = ClearType.CaptionStrong, color = Candy.Sky)
                    }
                }
                if (entry.failed && isLast) {
                    GlassPill(onClick = onRetry, height = 34.dp, horizontalPadding = 12.dp, label = stringResource(R.string.ai_retry)) {
                        Text(stringResource(R.string.ai_retry), style = ClearType.CaptionStrong, color = c.ink)
                    }
                }
            }
        }
    }
}

/** Halaman riwayat obrolan: cari, sematkan, ganti nama, hapus, dan buka lagi obrolan + dokumennya. */
@Composable
private fun AiHistoryPage(vm: AppViewModel, onBack: () -> Unit) {
    val c = clear
    val session = vm.aiSession
    var query by remember { mutableStateOf("") }
    var renameTarget by remember { mutableStateOf<AiChatMeta?>(null) }
    var deleteTarget by remember { mutableStateOf<AiChatMeta?>(null) }
    val dateFmt = remember { SimpleDateFormat("d MMM yyyy, HH.mm", Locale("id", "ID")) }
    val filtered = session.chats
        .filter { m -> query.isBlank() || m.title.contains(query, ignoreCase = true) || (m.docName?.contains(query, ignoreCase = true) == true) }
        .sortedWith(compareByDescending<AiChatMeta> { it.pinned }.thenByDescending { it.updatedAt })

    fun openChat(meta: AiChatMeta) {
        when {
            meta.docPath == null -> {
                if (session.attachedDocPath != null) session.detach()
                session.openChat(meta)
            }
            meta.docPath == session.attachedDocPath -> session.openChat(meta)
            else -> {
                session.requestChat(meta.id)
                vm.open(Uri.parse(meta.docPath))
            }
        }
        onBack()
    }

    Column(Modifier.fillMaxSize()) {
        Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars))
        Spacer(Modifier.height(14.dp))
        ScreenHeader(null, stringResource(R.string.ai_history), onBack = onBack)
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 22.dp, vertical = 6.dp)
                .glass(Capsule, c.glass, blur = 3.dp, lensHeight = 12.dp, lensAmount = 22.dp)
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            BasicTextField(
                value = query,
                onValueChange = { query = it },
                modifier = Modifier.fillMaxWidth(),
                textStyle = ClearType.Body.copy(color = c.ink),
                cursorBrush = SolidColor(c.ink),
                singleLine = true,
                decorationBox = { inner ->
                    Box {
                        if (query.isEmpty()) Text(stringResource(R.string.ai_history_search), style = ClearType.Body, color = c.inkSoft)
                        inner()
                    }
                },
            )
        }
        if (filtered.isEmpty()) {
            Text(
                stringResource(R.string.ai_history_empty),
                style = ClearType.Body, color = c.inkSoft,
                modifier = Modifier.padding(horizontal = 30.dp, vertical = 16.dp),
            )
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(horizontal = 22.dp, vertical = 10.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                items(filtered, key = { it.id }) { meta ->
                    Column(
                        Modifier.fillMaxWidth().wash(RoundedRectangle(18.dp), c.capsule)
                            .pressable({ openChat(meta) })
                            .padding(horizontal = 16.dp, vertical = 12.dp),
                    ) {
                        Text(meta.title.ifBlank { stringResource(R.string.ai_history_free) }, style = ClearType.BodyStrong, color = c.ink, maxLines = 2)
                        Spacer(Modifier.height(4.dp))
                        Text(
                            (meta.docName ?: stringResource(R.string.ai_history_free)) + " · " + dateFmt.format(Date(meta.updatedAt)) +
                                if (meta.pinned) " · " + stringResource(R.string.ai_pinned) else "",
                            style = ClearType.Caption, color = c.inkSoft,
                        )
                        Spacer(Modifier.height(8.dp))
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            GlassPill(
                                onClick = { session.pinChat(meta, !meta.pinned) }, height = 32.dp, horizontalPadding = 12.dp,
                                label = stringResource(if (meta.pinned) R.string.ai_unpin else R.string.ai_pin),
                            ) {
                                Text(stringResource(if (meta.pinned) R.string.ai_unpin else R.string.ai_pin), style = ClearType.CaptionStrong, color = c.ink)
                            }
                            GlassPill(onClick = { renameTarget = meta }, height = 32.dp, horizontalPadding = 12.dp, label = stringResource(R.string.ai_rename)) {
                                Text(stringResource(R.string.ai_rename), style = ClearType.CaptionStrong, color = c.ink)
                            }
                            GlassPill(onClick = { deleteTarget = meta }, height = 32.dp, horizontalPadding = 12.dp, label = stringResource(R.string.ai_delete_chat)) {
                                Text(stringResource(R.string.ai_delete_chat), style = ClearType.CaptionStrong, color = Candy.Tomato)
                            }
                        }
                    }
                }
            }
        }
    }

    renameTarget?.let { meta ->
        var name by remember(meta.id) { mutableStateOf(meta.title) }
        GlassDialog(
            onDismiss = { renameTarget = null }, title = stringResource(R.string.ai_rename_title),
            confirm = stringResource(R.string.common_save),
            onConfirm = { session.renameChat(meta, name); renameTarget = null },
            dismiss = stringResource(R.string.common_cancel),
        ) {
            GlassTextField(name, { name = it }, stringResource(R.string.ai_rename_title))
        }
    }
    deleteTarget?.let { meta ->
        GlassDialog(
            onDismiss = { deleteTarget = null }, title = stringResource(R.string.ai_delete_chat),
            confirm = stringResource(R.string.ai_delete_chat),
            onConfirm = { session.deleteChat(meta); deleteTarget = null },
            dismiss = stringResource(R.string.common_cancel),
        ) {
            Text(stringResource(R.string.ai_delete_chat_body), style = ClearType.Body, color = c.inkSoft)
        }
    }
}

/**
 * Halaman pemilih berkas penuh bergaya layar Berkas (cari, tab Terakhir/Berbintang/Di
 * Perangkat, kelompok per hari, panel cuci), dengan pilih sekali banyak. Dipakai dua peran:
 * dokumen konteks (chip Tambah) dan dokumen pedoman (tombol +). Yang sudah menempel dan
 * dokumen yang sedang terbuka disembunyikan dari daftar.
 */
@Composable
private fun AiPickerPage(
    vm: AppViewModel,
    forReference: Boolean,
    excludeKeys: Set<String>,
    onBack: () -> Unit,
    onConfirm: (List<FileRow>) -> Unit,
) {
    val c = clear
    var tab by remember { mutableStateOf(0) }
    var query by remember { mutableStateOf("") }
    val selected = remember { mutableStateMapOf<String, FileRow>() }
    // Pindai berkas perangkat sekali saat halaman dibuka, supaya tab Di Perangkat dan
    // pencarian langsung lengkap (pola yang sama seperti layar Berkas saat tabnya dibuka).
    LaunchedEffect(Unit) { vm.scanLocal() }
    LaunchedEffect(tab) { if (tab == 2) vm.scanLocal() }

    val rows: List<FileRow> = when {
        query.isNotBlank() -> vm.searchFiles(query)
            .sortedWith(compareBy<FileRow> { !it.name.lowercase().startsWith(query.trim().lowercase()) }.thenByDescending { it.date })
        tab == 0 -> vm.recentRows()
        tab == 1 -> vm.starredRows()
        else -> vm.localRows()
    }.filter { it.key !in excludeKeys }

    val bucketLabels = listOf(
        stringResource(R.string.files_today), stringResource(R.string.common_yesterday),
        stringResource(R.string.files_this_week), stringResource(R.string.files_earlier),
    )
    val internal = stringResource(R.string.home_internal_storage)

    Column(Modifier.fillMaxSize()) {
        Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars))
        Spacer(Modifier.height(14.dp))
        ScreenHeader(
            null,
            stringResource(if (forReference) R.string.ai_pick_title_ref else R.string.ai_pick_title_context),
            onBack = onBack,
        )
        // kapsul pencarian kaca, bahasa yang sama dengan layar Berkas
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 22.dp, vertical = 6.dp)
                .glass(Capsule, c.glass, blur = 3.dp, lensHeight = 12.dp, lensAmount = 22.dp)
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            GlassIcon(LucideIcons.Search, null, Modifier.size(19.dp), tint = c.inkSoft)
            Spacer(Modifier.width(10.dp))
            BasicTextField(
                value = query,
                onValueChange = { query = it },
                modifier = Modifier.weight(1f),
                textStyle = ClearType.Body.copy(color = c.ink),
                cursorBrush = SolidColor(c.ink),
                singleLine = true,
                decorationBox = { inner ->
                    Box {
                        if (query.isEmpty()) Text(stringResource(R.string.home_search_your_files), style = ClearType.Body, color = c.inkSoft)
                        inner()
                    }
                },
            )
            if (query.isNotEmpty()) {
                Box(Modifier.size(30.dp).pressable({ query = "" }), contentAlignment = Alignment.Center) {
                    GlassIcon(LucideIcons.CircleX, null, Modifier.size(18.dp), tint = c.inkSoft)
                }
            }
        }
        if (query.isBlank()) {
            GlassTabs(
                tabs = listOf(
                    GlassTab(stringResource(R.string.home_list_recent), accent = Candy.Tomato),
                    GlassTab(stringResource(R.string.home_list_starred), accent = Candy.Tangerine),
                    GlassTab(stringResource(R.string.home_list_on_device), accent = Candy.Sky),
                ),
                selected = tab, onSelect = { tab = it },
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp).fillMaxWidth(),
                height = 48.dp,
            )
        }
        if (rows.isEmpty()) {
            Text(
                stringResource(R.string.ai_pick_empty),
                style = ClearType.Body, color = c.inkSoft,
                modifier = Modifier.padding(horizontal = 30.dp, vertical = 16.dp),
            )
            Box(Modifier.weight(1f))
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f).fillMaxWidth(),
                contentPadding = PaddingValues(horizontal = 16.dp, vertical = 6.dp),
            ) {
                val groups: List<Pair<String?, List<FileRow>>> = when {
                    query.isNotBlank() -> listOf(null to rows)
                    tab == 0 -> rows.groupBy { dayBucket(it.date) }.toSortedMap()
                        .map { (bucket, list) -> bucketLabels[bucket] to list }
                    tab == 2 -> rows.groupBy { it.location }
                        .map { (folder, list) -> folder.ifEmpty { internal } to list }
                    else -> listOf(null to rows)
                }
                for ((label, list) in groups) {
                    if (label != null) {
                        item(key = "t-" + label) {
                            Row(
                                Modifier.fillMaxWidth().padding(start = 8.dp, end = 8.dp, top = 18.dp, bottom = 8.dp),
                                verticalAlignment = Alignment.Bottom,
                            ) {
                                Text(label, style = ClearType.Title, color = c.ink, modifier = Modifier.weight(1f))
                                Text(list.size.toString(), style = ClearType.Number, color = c.inkFaint)
                            }
                        }
                    }
                    list.chunked(20).forEachIndexed { i, chunk ->
                        item(key = "p-" + (label ?: "all") + "-" + i) {
                            WashPanel(Modifier.padding(vertical = 4.dp).fillMaxWidth()) {
                                Spacer(Modifier.height(4.dp))
                                chunk.forEach { row ->
                                    PickerRow(
                                        row = row,
                                        ticked = row.key in selected,
                                        onToggle = {
                                            if (row.key in selected) selected.remove(row.key) else selected[row.key] = row
                                        },
                                    )
                                }
                                Spacer(Modifier.height(4.dp))
                            }
                        }
                    }
                }
            }
        }
        // bar konfirmasi: jumlah pilihan selalu terlihat, tombol mati bila belum ada pilihan
        Row(
            Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            GlassPill(
                onClick = { if (selected.isNotEmpty()) onConfirm(selected.values.toList()) },
                height = 48.dp, horizontalPadding = 20.dp,
                surface = if (selected.isEmpty()) c.glass else Candy.Sky,
                label = stringResource(R.string.ai_pick_add, selected.size),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    stringResource(R.string.ai_pick_add, selected.size),
                    style = ClearType.BodyStrong,
                    color = if (selected.isEmpty()) c.inkSoft else androidx.compose.ui.graphics.Color.White,
                )
            }
        }
    }
}

/** Satu baris berkas di halaman pemilih: ikon jenis, nama, kapan dan ukuran, lingkaran centang. */
@Composable
private fun PickerRow(row: FileRow, ticked: Boolean, onToggle: () -> Unit) {
    val c = clear
    val kind = DocumentTypes.kindOf(row.name)
    Row(
        Modifier.fillMaxWidth()
            .background(if (ticked) Candy.Sky.copy(alpha = 0.14f) else androidx.compose.ui.graphics.Color.Transparent)
            .pressable(onToggle)
            .padding(start = 14.dp, end = 10.dp, top = 10.dp, bottom = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier.size(40.dp).clip(RoundedRectangle(10.dp)).background(c.capsule),
            contentAlignment = Alignment.Center,
        ) { GlassIcon(kind.icon, null, Modifier.size(20.dp), tint = c.inkSoft) }
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(displayName(row.name), style = ClearType.BodyStrong, color = c.ink, maxLines = 2)
            Spacer(Modifier.height(2.dp))
            Text(
                listOfNotNull(whenLabel(row.date), row.size.takeIf { it > 0 }?.let { formatSize(it) }).joinToString("  ·  "),
                style = ClearType.Caption, color = c.inkSoft, maxLines = 1,
            )
        }
        if (row.starred) {
            GlassIcon(LucideIcons.Star, null, Modifier.size(16.dp), tint = Candy.Lemon)
            Spacer(Modifier.width(6.dp))
        }
        Box(Modifier.size(40.dp), contentAlignment = Alignment.Center) {
            Box(
                Modifier.size(24.dp).clip(CircleShape)
                    .background(if (ticked) Candy.Sky else androidx.compose.ui.graphics.Color.Transparent)
                    .border(2.dp, if (ticked) Candy.Sky else c.inkFaint, CircleShape),
                contentAlignment = Alignment.Center,
            ) {
                if (ticked) GlassIcon(LucideIcons.Check, null, Modifier.size(14.dp), tint = androidx.compose.ui.graphics.Color.White)
            }
        }
    }
}
