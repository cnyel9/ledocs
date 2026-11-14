package com.ledocs.app.ai

import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.foundation.layout.windowInsetsTopHeight
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.ledocs.app.R
import com.ledocs.app.ui.AppViewModel
import com.ledocs.app.ui.LucideIcons
import com.ledocs.app.ui.desk.ScreenHeader
import com.ledocs.app.ui.glass.Capsule
import com.ledocs.app.ui.glass.GlassCircle
import com.ledocs.app.ui.glass.GlassDialog
import com.ledocs.app.ui.glass.GlassIcon
import com.ledocs.app.ui.glass.GlassPill
import com.ledocs.app.ui.glass.RoundedRectangle
import com.ledocs.app.ui.glass.glass
import com.ledocs.app.ui.glass.pressable
import com.ledocs.app.ui.glass.wash
import com.ledocs.app.ui.theme.Candy
import com.ledocs.app.ui.theme.ClearType
import com.ledocs.app.ui.theme.clear
import java.net.URL

/** Apakah perangkat sedang punya jalur internet (bukan sekadar tersambung ke jaringan). */
private fun onlineNow(context: android.content.Context): Boolean = runCatching {
    val cm = context.getSystemService(ConnectivityManager::class.java) ?: return false
    val caps = cm.getNetworkCapabilities(cm.activeNetwork) ?: return false
    caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
}.getOrDefault(false)

/**
 * Layar Asisten AI: menggantikan navigasi di tempatnya (dock hilang selama layar ini terbuka,
 * bar ketiknya duduk di posisi yang sama). Dokumen yang sedang terbuka menempel sendiri sebagai
 * konteks; tanpa dokumen, ini obrolan bebas. Sesi hidup di AppViewModel, jadi menutup layar
 * tidak membunuh stream yang sedang berjalan.
 */
@Composable
fun AiScreen(vm: AppViewModel) {
    val c = clear
    val context = LocalContext.current
    val store = vm.aiStore
    val session = vm.aiSession
    val doc = vm.document
    BackHandler { vm.aiOpen = false }
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
    val provider = store.active
    // Provider lokal (server di perangkat sendiri) tidak butuh internet sama sekali.
    val reachable = online || (provider != null && AiClient.isLoopback(provider.baseUrl))
    LaunchedEffect(provider?.id) { if (provider == null) pendingAction = null }

    fun hostOf(p: AiProvider): String = runCatching { URL(p.baseUrl).host }.getOrDefault(p.baseUrl)

    /** Gerbang persetujuan: kiriman pertama yang membawa teks dokumen ke provider ini wajib izin. */
    fun withConsent(action: () -> Unit) {
        val p = provider ?: return
        val docAttached = session.context is AiContext.Attached || session.preparing
        if (docAttached && !store.consented(p.id, hostOf(p), session.attachedDocId)) {
            pendingAction = action
        } else {
            action()
        }
    }

    fun trySend() {
        val text = input.trim()
        if (text.isEmpty() || session.running || session.preparing) return
        withConsent {
            input = ""
            session.send(text)
        }
    }

    fun retryWithConsent(entry: AiEntry) {
        withConsent { session.retry(entry) }
    }

    Column(Modifier.fillMaxSize()) {
        Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars))
        Spacer(Modifier.height(14.dp))
        ScreenHeader(null, stringResource(R.string.ai_title), onBack = { vm.aiOpen = false }) {
            GlassCircle(onClick = { session.newChat() }, size = 40.dp, label = stringResource(R.string.ai_new_chat)) {
                GlassIcon(LucideIcons.MessageSquarePlus, null, Modifier.size(19.dp))
            }
        }
        // pil provider: nama dan model aktif, ketuk untuk mengganti
        Row(Modifier.fillMaxWidth().padding(horizontal = 22.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
            GlassPill(onClick = { picker = true }, height = 40.dp, horizontalPadding = 14.dp, label = provider?.name) {
                GlassIcon(LucideIcons.Sparkles, null, Modifier.size(16.dp), tint = Candy.Sky)
                Spacer(Modifier.width(8.dp))
                Text(
                    if (provider == null) stringResource(R.string.ai_open_settings) else "${provider.name} · ${provider.model}",
                    style = ClearType.CaptionStrong, color = c.ink,
                )
            }
            Spacer(Modifier.weight(1f))
            if (session.running) Text(stringResource(R.string.ai_thinking), style = ClearType.Caption, color = c.inkSoft)
        }
        // chip konteks dokumen
        if (doc != null) {
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 22.dp, vertical = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                GlassIcon(LucideIcons.FileText, null, Modifier.size(16.dp), tint = Candy.Sky)
                Spacer(Modifier.width(8.dp))
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
                    GlassCircle(onClick = { session.detach() }, size = 30.dp, label = stringResource(R.string.ai_detach)) {
                        GlassIcon(LucideIcons.X, null, Modifier.size(14.dp))
                    }
                }
            }
        }

        val listState = rememberLazyListState()
        LaunchedEffect(session) {
            snapshotFlow { session.entries.size to (session.entries.lastOrNull()?.text?.length ?: 0) }
                .collect {
                    if (session.entries.isNotEmpty() && !listState.isScrollInProgress) {
                        listState.animateScrollToItem(session.entries.size - 1)
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
                    GlassIcon(LucideIcons.Sparkles, null, Modifier.size(40.dp), tint = Candy.Sky)
                    Spacer(Modifier.height(14.dp))
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
                    GlassIcon(LucideIcons.Globe, null, Modifier.size(40.dp), tint = c.inkSoft)
                    Spacer(Modifier.height(14.dp))
                    Text(stringResource(R.string.ai_offline), style = ClearType.Title, color = c.ink)
                    Spacer(Modifier.height(8.dp))
                    Text(stringResource(R.string.ai_offline_body), style = ClearType.Body, color = c.inkSoft)
                }
                session.entries.isEmpty() -> Column(
                    Modifier.fillMaxSize().padding(horizontal = 22.dp, vertical = 10.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
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
                    items(session.entries) { entry -> EntryRow(entry, onRetry = { retryWithConsent(entry) }) }
                }
            }
        }

        // bar ketik: kapsul kaca yang sama bahasanya dengan dock, duduk di posisi dock
        if (provider != null) {
            if (!reachable) {
                Row(
                    Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = 16.dp, vertical = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    GlassIcon(LucideIcons.Globe, null, Modifier.size(18.dp), tint = c.inkSoft)
                    Spacer(Modifier.width(10.dp))
                    Text(stringResource(R.string.ai_offline), style = ClearType.BodyStrong, color = c.inkSoft)
                }
            } else {
                Row(
                    Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = 16.dp, vertical = 12.dp)
                        .glass(Capsule, c.glass, blur = 3.dp, lensHeight = 14.dp, lensAmount = 26.dp)
                        .drawBehind { drawRect(c.capsule, alpha = 0.9f) }
                        .padding(start = 16.dp, end = 6.dp, top = 6.dp, bottom = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
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

    if (picker) {
        GlassDialog(
            onDismiss = { picker = false }, title = stringResource(R.string.ai_providers_title), icon = LucideIcons.Sparkles, accent = Candy.Sky,
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

    pendingAction?.let { action ->
        val p = provider
        if (p != null) {
            val host = hostOf(p)
            GlassDialog(
                onDismiss = { pendingAction = null }, title = stringResource(R.string.ai_consent_title), icon = LucideIcons.Sparkles, accent = Candy.Sky,
                confirm = stringResource(R.string.ai_consent_send),
                onConfirm = {
                    store.consent(p.id, host, session.attachedDocId)
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

@Composable
private fun EntryRow(entry: AiEntry, onRetry: () -> Unit) {
    val c = clear
    if (entry.role == "user") {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
            Box(
                // Gelembung di daftar yang bergeser memakai wash (pola codebase untuk permukaan
                // yang scroll), bukan backdrop blur asli yang mahal per frame.
                Modifier.fillMaxWidth(0.85f).wash(Capsule, c.capsule)
                    .padding(horizontal = 16.dp, vertical = 10.dp),
            ) { SelectionContainer { Text(entry.text, style = ClearType.Body, color = c.ink) } }
        }
        return
    }
    Column(Modifier.fillMaxWidth()) {
        Row(verticalAlignment = Alignment.Top) {
            Box(Modifier.size(26.dp).clip(CircleShape).background(Candy.Sky.copy(alpha = 0.18f)), contentAlignment = Alignment.Center) {
                GlassIcon(LucideIcons.Sparkles, null, Modifier.size(14.dp), tint = Candy.Sky)
            }
            Spacer(Modifier.width(10.dp))
            Column(Modifier.weight(1f)) {
                if (entry.text.isNotEmpty()) SelectionContainer { Text(entry.text, style = ClearType.Body, color = c.ink) }
                if (entry.stopped) Text(stringResource(R.string.ai_stopped), style = ClearType.Caption, color = c.inkSoft)
                if (entry.interrupted) Text(stringResource(R.string.ai_interrupted), style = ClearType.Caption, color = Candy.Tangerine)
                if (entry.failed) {
                    entry.error?.let { Text(it, style = ClearType.Body, color = Candy.Tomato) }
                    Spacer(Modifier.height(8.dp))
                    GlassPill(onClick = onRetry, height = 38.dp, horizontalPadding = 12.dp, label = stringResource(R.string.ai_retry)) {
                        GlassIcon(LucideIcons.RotateCcw, null, Modifier.size(15.dp), tint = c.ink)
                        Spacer(Modifier.width(6.dp))
                        Text(stringResource(R.string.ai_retry), style = ClearType.CaptionStrong, color = c.ink)
                    }
                }
            }
        }
    }
}
