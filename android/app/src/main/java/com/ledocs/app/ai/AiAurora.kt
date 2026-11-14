package com.ledocs.app.ai

import android.provider.Settings
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Canvas
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.drawscope.CanvasDrawScope
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.IntSize
import kotlinx.coroutines.isActive
import kotlin.math.PI
import kotlin.math.sin

/** One bloom of the AI aurora: where it lives, how big and bright it is, and how it wanders. */
private data class AiBloom(
    val anchor: Offset,
    val color: Color,
    val strength: Float,
    val radiusFrac: Float,
    val periodXms: Double,
    val periodYms: Double,
    val phaseX: Double,
    val phaseY: Double,
    val ampX: Float,
    val ampY: Float,
)

// Monochrome only, silver to white on black. Periods are pairwise incommensurate (47/61/79/97/113 s
// on x, 59/73/89/107/127 s on y) with unrelated phases, so the combined drift does not visibly
// repeat on any human timescale.
private val AiBlooms = listOf(
    AiBloom(Offset(0.12f, 0.16f), Color(0xFFFFFFFF), 0.22f, 0.72f, 47_000.0, 59_000.0, 0.0, 1.7, 0.050f, 0.040f),
    AiBloom(Offset(0.88f, 0.30f), Color(0xFFC7C7CC), 0.16f, 0.62f, 61_000.0, 73_000.0, 2.1, 0.4, 0.040f, 0.060f),
    AiBloom(Offset(0.68f, 0.84f), Color(0xFF8E8E93), 0.13f, 0.68f, 79_000.0, 89_000.0, 4.0, 2.9, 0.060f, 0.035f),
    AiBloom(Offset(0.24f, 0.72f), Color(0xFF48484A), 0.10f, 0.55f, 97_000.0, 107_000.0, 1.2, 5.1, 0.035f, 0.050f),
    AiBloom(Offset(0.52f, 0.04f), Color(0xFFFFFFFF), 0.18f, 0.75f, 113_000.0, 127_000.0, 5.5, 3.3, 0.070f, 0.045f),
)

private const val TAU = 2.0 * PI

/**
 * The AI chat's backdrop: pure black with five monochrome blooms of silver-white light drifting on
 * very slow sine paths. Painted like the glass wallpaper, into one bitmap an eighth of the screen
 * each way that is repainted in place per frame and drawn scaled up, so a frame costs a small image
 * instead of five full-screen gradients. Static when the system animator scale is 0.
 */
@Composable
fun AiAuroraBackground(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val reduceMotion = remember {
        Settings.Global.getFloat(
            context.contentResolver,
            Settings.Global.ANIMATOR_DURATION_SCALE,
            1f,
        ) == 0f
    }
    var frameMs by remember { mutableLongStateOf(0L) }
    if (!reduceMotion) {
        LaunchedEffect(Unit) {
            var start = -1L
            while (isActive) {
                withFrameNanos { nanos ->
                    if (start < 0L) start = nanos
                    frameMs = (nanos - start) / 1_000_000L
                }
            }
        }
    }
    val bitmap = remember { arrayOfNulls<ImageBitmap>(1) }
    Box(
        modifier.drawWithCache {
            val w = (size.width / 8f).toInt().coerceAtLeast(1)
            val h = (size.height / 8f).toInt().coerceAtLeast(1)
            // the base is opaque black, so repainting over the last frame needs no clear
            val image = bitmap[0]?.takeIf { it.width == w && it.height == h } ?: ImageBitmap(w, h).also { bitmap[0] = it }
            val small = Size(w.toFloat(), h.toFloat())
            // reading the frame time here makes the cache follow it, one repaint per frame
            val t = frameMs.toDouble()
            CanvasDrawScope().draw(this, layoutDirection, Canvas(image), small) {
                drawRect(Color.Black)
                val short = small.minDimension
                for (b in AiBlooms) {
                    val cx = (b.anchor.x + b.ampX * sin(TAU * t / b.periodXms + b.phaseX).toFloat()) * small.width
                    val cy = (b.anchor.y + b.ampY * sin(TAU * t / b.periodYms + b.phaseY).toFloat()) * small.height
                    val center = Offset(cx, cy)
                    val r = b.radiusFrac * short
                    if (r <= 1f) continue
                    drawCircle(
                        Brush.radialGradient(
                            listOf(
                                b.color.copy(alpha = b.strength),
                                b.color.copy(alpha = b.strength * 0.55f),
                                b.color.copy(alpha = b.strength * 0.18f),
                                Color.Transparent,
                            ),
                            center, r,
                        ),
                        radius = r, center = center,
                    )
                }
            }
            onDrawBehind { drawImage(image, dstSize = IntSize(size.width.toInt(), size.height.toInt()), filterQuality = FilterQuality.Low) }
        },
    )
}
