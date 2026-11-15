package com.ledocs.app.update

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.ledocs.app.R
import com.ledocs.app.ai.MarkdownText
import com.ledocs.app.ui.LucideIcons
import com.ledocs.app.ui.glass.Capsule
import com.ledocs.app.ui.glass.GlassDialog
import com.ledocs.app.ui.theme.Candy
import com.ledocs.app.ui.theme.ClearType
import com.ledocs.app.ui.theme.clear
import com.ledocs.app.ui.viewer.formatSize

/**
 * Kartu pembaruan yang muncul di atas layar mana pun saat pemeriksa menemukan
 * rilis yang lebih baru: versi, catatan rilis, lalu Unduh & Pasang. Selama
 * unduhan berjalan tombol berubah fungsi menjadi pemberhentian lewat tombol
 * Nanti yang memanggil batal unduh.
 */
@Composable
fun UpdateDialog(state: UpdateState) {
    val update = state.info
    if (!state.visible || update == null) return
    val c = clear
    GlassDialog(
        onDismiss = { if (state.downloading) state.cancelDownload() else state.dismiss() },
        title = stringResource(R.string.update_available_title),
        icon = LucideIcons.FileDown,
        accent = Candy.Sky,
        confirm = stringResource(
            if (state.downloaded != null) R.string.update_install else R.string.update_download_install,
        ),
        onConfirm = { if (state.downloaded != null) state.install() else state.download() },
        confirmEnabled = !state.downloading,
        dismiss = stringResource(if (state.downloading) R.string.common_cancel else R.string.update_later),
        onDismissButton = { if (state.downloading) state.cancelDownload() else state.dismiss() },
        dismissible = true,
    ) {
        Text(
            stringResource(R.string.update_version_line, update.version, formatSize(update.sizeBytes)),
            style = ClearType.BodyStrong,
            color = c.ink,
        )
        Spacer(Modifier.height(10.dp))
        Column(
            Modifier
                .fillMaxWidth()
                .heightIn(max = 260.dp)
                .verticalScroll(rememberScrollState()),
        ) {
            if (update.notes.isBlank()) {
                Text(stringResource(R.string.update_notes_empty), style = ClearType.Body, color = c.inkSoft)
            } else {
                MarkdownText(update.notes)
            }
        }
        if (state.downloading) {
            Spacer(Modifier.height(14.dp))
            Box(
                Modifier
                    .fillMaxWidth()
                    .height(6.dp)
                    .clip(Capsule)
                    .background(c.line),
            ) {
                Box(
                    Modifier
                        .fillMaxWidth(state.progress)
                        .height(6.dp)
                        .clip(Capsule)
                        .background(Candy.Sky),
                )
            }
            Spacer(Modifier.height(6.dp))
            Text(
                stringResource(R.string.update_downloading, (state.progress * 100).toInt()),
                style = ClearType.Caption,
                color = c.inkSoft,
            )
        }
        state.errorRes?.let { res ->
            Spacer(Modifier.height(10.dp))
            Text(stringResource(res), style = ClearType.Body, color = Candy.Tomato)
        }
    }
}
