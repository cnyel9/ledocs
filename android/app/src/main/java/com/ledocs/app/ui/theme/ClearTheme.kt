package com.ledocs.app.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.sp
import com.ledocs.app.R
import com.ledocs.app.ui.ThemeChoice

/**
 * The accents: Apple's system hues, one per tool, the same in both themes. Icons, toggles, the
 * vivid buttons. Surfaces use the softer [CardTone]s instead.
 */
object Candy {
    val Tomato = Color(0xFFFF5B4F)
    val Tangerine = Color(0xFFFF9F0A)
    val Lemon = Color(0xFFFFC21A)
    val Lime = Color(0xFF34C759)
    val Mint = Color(0xFF00BFB3)
    val Sky = Color(0xFF0A84FF)
    val Grape = Color(0xFFA6A6AF)
    val Bubblegum = Color(0xFFDEDEE4)

    /** The chosen chip's pastel lemon. */
    val Chip = Color(0xFFFFFFFF)
    /** A note's title blue. */
    val InkBlue = Color(0xFF45454D)

    val All = listOf(Tomato, Tangerine, Lemon, Lime, Mint, Sky, Grape, Bubblegum)
}

/** A pastel card's paint: a soft vertical gradient, the ink on it and its own accent (titles, serif words). */
@Immutable
data class CardTone(val top: Color, val bottom: Color, val ink: Color, val accent: Color)

/** The card colours, by role. */
enum class Tone { Sage, Paper, Lavender, Butter, Orchid, Blush, Sky, Peach }

@Immutable
data class ClearColors(
    val dark: Boolean,
    /** Behind everything: the wallpaper's gradient, top to bottom. */
    val wall: List<Color>,
    /** Soft light pooled on the wallpaper: colour and strength. */
    val blooms: List<Pair<Color, Float>>,
    /** Big titles straight on the wallpaper. */
    val hero: Color,
    /** The glow behind [hero] titles. */
    val heroGlow: Color,
    val ink: Color,
    val inkSoft: Color,
    val inkFaint: Color,
    val line: Color,
    /** The wash on chrome glass (dock, bars, buttons, chips). */
    val glass: Color,
    /** The rim of an outlined glass chip. */
    val rim: Color,
    /** The heavier frost menus, sheets and dialogs use so their text stays legible. */
    val frost: Color,
    /** Behind document pages. */
    val desk: Color,
    /** The dark glass of the floating create capsule. */
    val capsule: Color,
    val tones: Map<Tone, CardTone>,
) {
    /** The canvas colour (the wallpaper's middle), for places that need one flat colour. */
    val canvas: Color get() = wall[wall.size / 2]

    fun tone(t: Tone): CardTone = tones.getValue(t)
}

private val InkLight = Color(0xFF172033)
private val InkDark = Color(0xFFF4F4F5)

private val LightTones = mapOf(
    Tone.Sage to CardTone(Color(0xFFF2F2F4), Color(0xFFE6E6EA), InkLight, Color(0xFF3F3F46)),
    Tone.Paper to CardTone(Color(0xFFFFFFFF).copy(alpha = 0.88f), Color(0xFFF1F1F4).copy(alpha = 0.82f), InkLight, Color(0xFF3F3F46)),
    Tone.Lavender to CardTone(Color(0xFFF2F2F4), Color(0xFFE6E6EA), InkLight, Color(0xFF3F3F46)),
    Tone.Butter to CardTone(Color(0xFFF2F2F4), Color(0xFFE6E6EA), InkLight, Color(0xFF3F3F46)),
    Tone.Orchid to CardTone(Color(0xFFF2F2F4), Color(0xFFE6E6EA), InkLight, Color(0xFF3F3F46)),
    Tone.Blush to CardTone(Color(0xFFF2F2F4), Color(0xFFE6E6EA), InkLight, Color(0xFF3F3F46)),
    Tone.Sky to CardTone(Color(0xFFF2F2F4), Color(0xFFE6E6EA), InkLight, Color(0xFF3F3F46)),
    Tone.Peach to CardTone(Color(0xFFF2F2F4), Color(0xFFE6E6EA), InkLight, Color(0xFF3F3F46)),
)

/**
 * At night the cards become jewels: saturated, deep, lit from above, with pale ink on them. (The
 * pastels merely darkened turned muddy against the midnight wallpaper.)
 */
private val DarkTones = mapOf(
    Tone.Sage to CardTone(Color(0xFF1E1E22), Color(0xFF111113), InkDark, Color.White),
    Tone.Paper to CardTone(Color(0xFF1E1E22), Color(0xFF111113), InkDark, Color.White),
    Tone.Lavender to CardTone(Color(0xFF1E1E22), Color(0xFF111113), InkDark, Color.White),
    Tone.Butter to CardTone(Color(0xFF1E1E22), Color(0xFF111113), InkDark, Color.White),
    Tone.Orchid to CardTone(Color(0xFF1E1E22), Color(0xFF111113), InkDark, Color.White),
    Tone.Blush to CardTone(Color(0xFF1E1E22), Color(0xFF111113), InkDark, Color.White),
    Tone.Sky to CardTone(Color(0xFF1E1E22), Color(0xFF111113), InkDark, Color.White),
    Tone.Peach to CardTone(Color(0xFF1E1E22), Color(0xFF111113), InkDark, Color.White),
)

val LightClear = ClearColors(
    dark = false,
    wall = listOf(Color(0xFFD9D9DE), Color(0xFFE4E4E8), Color(0xFFD2D2D8)),
    blooms = listOf(
        Color(0xFFFFFFFF) to 0.62f,
        Color(0xFFEDEDF0) to 0.58f,
        Color(0xFFE2E2E7) to 0.36f,
        Color(0xFFC9C9D1) to 0.42f,
        Color(0xFFFFFFFF) to 0.20f,
    ),
    hero = Color.White,
    heroGlow = Color(0xFF3F3F46).copy(alpha = 0.22f),
    ink = InkLight,
    inkSoft = InkLight.copy(alpha = 0.64f),
    inkFaint = InkLight.copy(alpha = 0.40f),
    line = InkLight.copy(alpha = 0.08f),
    glass = Color.White.copy(alpha = 0.26f),
    rim = Color.White.copy(alpha = 0.70f),
    frost = Color(0xFFF4F4F6).copy(alpha = 0.82f),
    desk = Color(0xFFE7E7EB),
    capsule = Color(0xFF101014).copy(alpha = 0.34f),
    tones = LightTones,
)

val DarkClear = ClearColors(
    dark = true,
    // Midnight Aurora: deep indigo with cobalt, violet and a breath of magenta pooled in it
    wall = listOf(Color(0xFF000000), Color(0xFF000000), Color(0xFF000000)),
    blooms = emptyList(),
    hero = Color.White,
    heroGlow = Color.Transparent,
    ink = InkDark,
    inkSoft = InkDark.copy(alpha = 0.68f),
    inkFaint = InkDark.copy(alpha = 0.44f),
    line = InkDark.copy(alpha = 0.10f),
    glass = Color.White.copy(alpha = 0.08f),
    rim = Color.White.copy(alpha = 0.26f),
    frost = Color(0xFF101012).copy(alpha = 0.96f),
    desk = Color(0xFF000000),
    capsule = Color(0xFF000000).copy(alpha = 0.55f),
    tones = DarkTones,
)

val LocalClearColors = staticCompositionLocalOf { LightClear }

/** The colours in effect. */
val clear: ClearColors @Composable get() = LocalClearColors.current

/** Plus Jakarta Sans (OFL) at one weight of its variable axis. */
private fun jakarta(weight: Int, italic: Boolean = false) = FontFamily(
    Font(
        if (italic) R.font.jakarta_italic else R.font.jakarta,
        FontWeight(weight),
        if (italic) FontStyle.Italic else FontStyle.Normal,
        variationSettings = FontVariation.Settings(FontVariation.weight(weight)),
    ),
)

/** The whole Jakarta family, every weight the text styles ask for. */
val Jakarta: FontFamily = FontFamily(
    listOf(300, 400, 500, 600, 700, 800).flatMap { w ->
        listOf(
            Font(R.font.jakarta, FontWeight(w), FontStyle.Normal, variationSettings = FontVariation.Settings(FontVariation.weight(w))),
            Font(R.font.jakarta_italic, FontWeight(w), FontStyle.Italic, variationSettings = FontVariation.Settings(FontVariation.weight(w))),
        )
    },
)

/** Doto (OFL): the dot-matrix face of to-do lists. */
val Doto: FontFamily = FontFamily(
    Font(R.font.doto, FontWeight.Bold, variationSettings = FontVariation.Settings(FontVariation.weight(800), FontVariation.Setting("ROND", 60f))),
)

/** DM Serif Display (OFL): the editorial word on a collection's card ("Files", "Pdf"). */
val Serif: FontFamily = FontFamily(
    Font(R.font.dm_serif, FontWeight.Normal),
    Font(R.font.dm_serif_italic, FontWeight.Normal, FontStyle.Italic),
)

/** Fraunces (OFL): a soft serif. [soft] rounds its terminals (0..100); [wonk] 1 leans some letters (quirky), 0 keeps them upright. */
private fun fraunces(weight: Int, soft: Float, size: TextUnit, wonk: Float = 0f) = FontFamily(
    Font(
        R.font.fraunces, FontWeight(weight),
        variationSettings = FontVariation.Settings(
            FontVariation.weight(weight), FontVariation.opticalSizing(size),
            FontVariation.Setting("SOFT", soft), FontVariation.Setting("WONK", wonk),
        ),
    ),
)

/** Bricolage Grotesque (OFL): a grotesque with a width axis, from a tall condensed 75 to the normal 100. */
private fun bricolage(weight: Int, width: Float, size: TextUnit) = FontFamily(
    Font(
        R.font.bricolage, FontWeight(weight),
        variationSettings = FontVariation.Settings(
            FontVariation.weight(weight), FontVariation.width(width), FontVariation.opticalSizing(size),
        ),
    ),
)

private val Trim = LineHeightStyle(LineHeightStyle.Alignment.Center, LineHeightStyle.Trim.Both)

private fun display(family: FontFamily, size: TextUnit, line: TextUnit, tracking: TextUnit) =
    TextStyle(fontFamily = family, fontSize = size, lineHeight = line, letterSpacing = tracking, lineHeightStyle = Trim)

/**
 * The type scale, all Plus Jakarta Sans: heavy and tight for heroes and titles (Apple's display
 * sizes, a touch rounder), plain for reading. [Serif] and [Doto] are the playful accents on cards.
 */
object ClearType {
    val Hero = display(jakarta(700), 54.sp, 56.sp, (-1.4).sp)
    val Display = display(jakarta(600), 42.sp, 46.sp, (-1.0).sp)
    val Headline = display(jakarta(700), 26.sp, 30.sp, (-0.5).sp)
    val Title = display(jakarta(700), 18.sp, 22.sp, (-0.2).sp)
    val Tile = display(jakarta(700), 17.sp, 20.sp, (-0.2).sp)
    val Number = display(jakarta(700), 22.sp, 24.sp, 0.sp).copy(fontFeatureSettings = "tnum")

    /** A note card's title: bold, tight, in the note's blue. */
    val CardTitle = display(jakarta(800), 27.sp, 29.sp, (-0.6).sp)
    /** A collection card's word ("Files"). */
    val SerifWord = display(Serif, 52.sp, 52.sp, (-1.2).sp)
    /** A list card's title. */
    val DotTitle = display(Doto, 30.sp, 32.sp, 0.sp)

    // The desk's big title, one face per shelf, each in the voice of what it holds: a clean bold
    // grotesque for everything, an upright soft serif for notes, dots for lists, an editorial serif
    // for PDFs, a thin italic for pictures, a light wide grotesque for voices.
    // All six share one size and one line height, so switching chips never moves anything below the
    // title; the desk shrinks that one size (for every title alike) only if a translation is too wide.
    val DeskTitleSize = 44.sp
    val DeskTitleLine = 50.sp
    val DeskAll = display(bricolage(700, 100f, 96.sp), DeskTitleSize, DeskTitleLine, (-1.4).sp)
    val DeskNotes = display(fraunces(600, 50f, 72.sp), DeskTitleSize, DeskTitleLine, (-0.8).sp)
    val DeskTodo = display(Doto, DeskTitleSize, DeskTitleLine, 0.sp)
    val DeskPdfs = display(Serif, DeskTitleSize, DeskTitleLine, (-0.8).sp)
    val DeskImages = display(jakarta(300, italic = true), DeskTitleSize, DeskTitleLine, (-1.4).sp).copy(fontStyle = FontStyle.Italic)
    val DeskAudio = display(bricolage(300, 100f, 72.sp), DeskTitleSize, DeskTitleLine, (-1.0).sp)

    /** The Tools screen's title: Fraunces at its softest and quirkiest, a face no other screen uses. */
    val ToolsTitle = display(fraunces(700, 100f, 72.sp, wonk = 1f), DeskTitleSize, DeskTitleLine, (-0.8).sp)
    /** A tool tile's name, set like the desk's "Files" and "Pdf" words. */
    val ToolWord = display(Serif, 31.sp, 33.sp, (-0.6).sp)

    val Eyebrow = TextStyle(fontFamily = Jakarta, fontWeight = FontWeight.SemiBold, fontSize = 11.5.sp, lineHeight = 14.sp, letterSpacing = 1.4.sp)
    val Body = TextStyle(fontFamily = Jakarta, fontWeight = FontWeight.Normal, fontSize = 15.sp, lineHeight = 22.sp)
    val BodyStrong = Body.copy(fontWeight = FontWeight.SemiBold)
    val Label = TextStyle(fontFamily = Jakarta, fontWeight = FontWeight.Medium, fontSize = 13.sp, lineHeight = 18.sp)
    val Caption = TextStyle(fontFamily = Jakarta, fontWeight = FontWeight.Normal, fontSize = 11.5.sp, lineHeight = 15.sp)
}

/** A hero title's soft glow, so white type reads on the pale sky. */
fun TextStyle.glowing(c: ClearColors): TextStyle = copy(shadow = Shadow(c.heroGlow, Offset(0f, 2f), 22f))

@Composable
fun ClearTheme(choice: ThemeChoice, content: @Composable () -> Unit) {
    val dark = true // LeDocs: mode gelap permanen, latar hitam
    CompositionLocalProvider(LocalClearColors provides DarkClear, content = content)
}
