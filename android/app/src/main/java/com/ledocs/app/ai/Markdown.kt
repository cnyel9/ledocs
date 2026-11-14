package com.ledocs.app.ai

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.ledocs.app.ui.theme.ClearColors
import com.ledocs.app.ui.theme.ClearType
import com.ledocs.app.ui.theme.clear

/**
 * A small, dependency-free Markdown renderer for AI answers. Blocks are parsed line by line,
 * inline spans are built as an [AnnotatedString]. Malformed input degrades to plain paragraphs;
 * parsing never throws.
 */
@Composable
fun MarkdownText(text: String, modifier: Modifier = Modifier) {
    val c = clear
    val blocks = remember(text) { parseMarkdown(text) }
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        blocks.forEach { block -> MdBlockView(block, c) }
    }
}

// ---------------------------------------------------------------------------
// Block model and parser
// ---------------------------------------------------------------------------

private sealed interface MdBlock {
    data class Paragraph(val text: String) : MdBlock
    data class Heading(val level: Int, val text: String) : MdBlock
    data class Code(val code: String) : MdBlock
    data class Bullets(val items: List<String>) : MdBlock
    data class Numbers(val items: List<Pair<String, String>>) : MdBlock
    data class Quote(val text: String) : MdBlock
    data class Table(val lines: List<String>) : MdBlock
    data object Rule : MdBlock
}

private val HeadingRe = Regex("""^\s{0,3}(#{1,6})\s+(.*)$""")
private val BulletRe = Regex("""^\s*[-*+]\s+(.*)$""")
private val NumberRe = Regex("""^\s*(\d{1,9})[.)]\s+(.*)$""")
private val RuleRe = Regex("""^\s*(?:-{3,}|\*{3,}|_{3,})\s*$""")
private val TableSepRe = Regex("""^[|\s:\-]+$""")

private fun isBlockStart(line: String): Boolean {
    val t = line.trim()
    return t.startsWith("```") || t.startsWith("|") || t.startsWith(">") ||
        HeadingRe.matchEntire(line) != null || RuleRe.matchEntire(line) != null ||
        BulletRe.matchEntire(line) != null || NumberRe.matchEntire(line) != null
}

private fun parseMarkdown(text: String): List<MdBlock> {
    val lines = text.replace("\r\n", "\n").replace('\r', '\n').split('\n')
    val blocks = mutableListOf<MdBlock>()
    var i = 0
    while (i < lines.size) {
        val line = lines[i]
        val trimmed = line.trim()
        when {
            trimmed.isEmpty() -> i++

            trimmed.startsWith("```") -> {
                i++
                val code = mutableListOf<String>()
                while (i < lines.size && !lines[i].trimStart().startsWith("```")) {
                    code += lines[i]
                    i++
                }
                if (i < lines.size) i++ // closing fence (absent at EOF is fine)
                blocks += MdBlock.Code(code.joinToString("\n"))
            }

            HeadingRe.matchEntire(line) != null -> {
                val m = HeadingRe.matchEntire(line)!!
                blocks += MdBlock.Heading(m.groupValues[1].length, m.groupValues[2].trim())
                i++
            }

            RuleRe.matchEntire(line) != null -> {
                blocks += MdBlock.Rule
                i++
            }

            trimmed.startsWith("|") -> {
                val rows = mutableListOf<String>()
                while (i < lines.size && lines[i].trim().startsWith("|")) {
                    rows += lines[i].trim()
                    i++
                }
                val visible = rows.filterNot { TableSepRe.matches(it) }
                if (visible.isNotEmpty()) blocks += MdBlock.Table(visible)
            }

            trimmed.startsWith(">") -> {
                val quote = mutableListOf<String>()
                while (i < lines.size && lines[i].trimStart().startsWith(">")) {
                    var q = lines[i].trimStart().removePrefix(">")
                    if (q.startsWith(" ")) q = q.drop(1)
                    quote += q
                    i++
                }
                blocks += MdBlock.Quote(quote.joinToString("\n"))
            }

            BulletRe.matchEntire(line) != null -> {
                val items = mutableListOf<String>()
                while (i < lines.size) {
                    val m = BulletRe.matchEntire(lines[i]) ?: break
                    items += m.groupValues[1].trim()
                    i++
                }
                blocks += MdBlock.Bullets(items)
            }

            NumberRe.matchEntire(line) != null -> {
                val items = mutableListOf<Pair<String, String>>()
                while (i < lines.size) {
                    val m = NumberRe.matchEntire(lines[i]) ?: break
                    items += (m.groupValues[1] + ".") to m.groupValues[2].trim()
                    i++
                }
                blocks += MdBlock.Numbers(items)
            }

            else -> {
                val para = mutableListOf(line.trim())
                i++
                while (i < lines.size && lines[i].isNotBlank() && !isBlockStart(lines[i])) {
                    para += lines[i].trim()
                    i++
                }
                blocks += MdBlock.Paragraph(para.joinToString("\n"))
            }
        }
    }
    return blocks
}

// ---------------------------------------------------------------------------
// Inline spans: **bold**, *italic*, `code`, [label](url)
// ---------------------------------------------------------------------------

private fun parseInline(raw: String): AnnotatedString = buildAnnotatedString { appendInline(raw) }

private fun AnnotatedString.Builder.appendInline(s: String) {
    var i = 0
    while (i < s.length) {
        when {
            s.startsWith("***", i) || s.startsWith("___", i) -> {
                val marker = s.substring(i, i + 3)
                val close = if (i + 3 < s.length && !s[i + 3].isWhitespace()) s.indexOf(marker, i + 3) else -1
                if (close > i + 3) {
                    withStyle(SpanStyle(fontWeight = FontWeight.SemiBold, fontStyle = FontStyle.Italic)) {
                        appendInline(s.substring(i + 3, close))
                    }
                    i = close + 3
                } else { append(s[i]); i++ }
            }

            s.startsWith("**", i) || s.startsWith("__", i) -> {
                val marker = s.substring(i, i + 2)
                val close = if (i + 2 < s.length && !s[i + 2].isWhitespace()) s.indexOf(marker, i + 2) else -1
                if (close > i + 2) {
                    withStyle(SpanStyle(fontWeight = FontWeight.SemiBold)) {
                        appendInline(s.substring(i + 2, close))
                    }
                    i = close + 2
                } else { append(s[i]); i++ }
            }

            s[i] == '`' -> {
                val close = s.indexOf('`', i + 1)
                if (close > i + 1) {
                    withStyle(SpanStyle(fontFamily = FontFamily.Monospace)) {
                        append(s.substring(i + 1, close))
                    }
                    i = close + 1
                } else { append(s[i]); i++ }
            }

            s[i] == '*' || (s[i] == '_' && (i == 0 || !s[i - 1].isLetterOrDigit())) -> {
                val close = if (i + 1 < s.length && !s[i + 1].isWhitespace()) s.indexOf(s[i], i + 1) else -1
                if (close > i + 1) {
                    withStyle(SpanStyle(fontStyle = FontStyle.Italic)) {
                        appendInline(s.substring(i + 1, close))
                    }
                    i = close + 1
                } else { append(s[i]); i++ }
            }

            s[i] == '[' -> {
                val labelEnd = s.indexOf(']', i + 1)
                val urlEnd = if (labelEnd > i && labelEnd + 1 < s.length && s[labelEnd + 1] == '(') {
                    s.indexOf(')', labelEnd + 2)
                } else -1
                if (labelEnd > i + 1 && urlEnd > labelEnd + 2) {
                    withStyle(SpanStyle(textDecoration = TextDecoration.Underline)) {
                        appendInline(s.substring(i + 1, labelEnd))
                    }
                    i = urlEnd + 1
                } else { append(s[i]); i++ }
            }

            else -> { append(s[i]); i++ }
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

private val MonoStyle: TextStyle =
    ClearType.Body.copy(fontFamily = FontFamily.Monospace, fontSize = 13.sp, lineHeight = 19.sp)

private val TableStyle: TextStyle =
    ClearType.Body.copy(fontFamily = FontFamily.Monospace, fontSize = 12.sp, lineHeight = 17.sp)

private fun headingStyle(level: Int): TextStyle = when (level) {
    1 -> ClearType.Headline
    2 -> ClearType.Title
    3 -> ClearType.Tile
    else -> ClearType.BodyStrong
}

@Composable
private fun MdBlockView(block: MdBlock, c: ClearColors) {
    val tableScroll = rememberScrollState()
    when (block) {
        is MdBlock.Paragraph ->
            Text(parseInline(block.text), style = ClearType.Body, color = c.ink)

        is MdBlock.Heading ->
            Text(parseInline(block.text), style = headingStyle(block.level), color = c.ink)

        is MdBlock.Code -> Column(
            Modifier
                .fillMaxWidth()
                .background(c.glass.copy(alpha = 0.16f), RoundedCornerShape(10.dp))
                .padding(10.dp),
        ) {
            Text(block.code, style = MonoStyle, color = c.ink)
        }

        is MdBlock.Table -> Column(
            Modifier
                .fillMaxWidth()
                .background(c.glass.copy(alpha = 0.16f), RoundedCornerShape(10.dp))
                .padding(10.dp),
        ) {
            Column(Modifier.horizontalScroll(tableScroll)) {
                block.lines.forEach { row ->
                    Text(row, style = TableStyle, color = c.ink, softWrap = false)
                }
            }
        }

        is MdBlock.Bullets -> Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            block.items.forEach { item ->
                Row {
                    Text("•", style = ClearType.Body, color = c.inkSoft, modifier = Modifier.padding(end = 8.dp))
                    Text(parseInline(item), style = ClearType.Body, color = c.ink, modifier = Modifier.weight(1f))
                }
            }
        }

        is MdBlock.Numbers -> Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            block.items.forEach { (marker, item) ->
                Row {
                    Text(marker, style = ClearType.Body, color = c.inkSoft, modifier = Modifier.width(26.dp))
                    Text(parseInline(item), style = ClearType.Body, color = c.ink, modifier = Modifier.weight(1f))
                }
            }
        }

        is MdBlock.Quote -> Row(Modifier.height(IntrinsicSize.Min)) {
            Box(
                Modifier
                    .fillMaxHeight()
                    .width(3.dp)
                    .background(c.inkFaint, RoundedCornerShape(2.dp)),
            )
            Text(
                parseInline(block.text),
                style = ClearType.Body,
                color = c.inkSoft,
                modifier = Modifier.padding(start = 10.dp).weight(1f),
            )
        }

        is MdBlock.Rule -> Box(Modifier.fillMaxWidth().height(1.dp).background(c.line))
    }
}
