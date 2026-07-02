/**
 * Text preprocessor utility for TTS (Text-to-Speech).
 *
 * Provides markdown stripping, punctuation filtering, and sentence splitting
 * to prepare raw agent messages for natural-sounding sequential speech output.
 *
 * **Important:** These functions operate on raw text **before** markdown
 * rendering — they do **not** handle HTML that markdown-it would add.
 *
 * @module voice/utils/text-preprocessor
 */

/**
 * A single sentence segment ready for TTS playback.
 */
export interface TextSegment {
  /** The preprocessed text of this segment. */
  text: string
  /** Zero-based index among all segments of the source message. */
  index: number
  /** Whether this is the final segment in the sequence. */
  isLast: boolean
}

// ────────────────────────────────────────────────────────────────────────────
// Unicode / regex constants
// ────────────────────────────────────────────────────────────────────────────

/**
 * Common emoji Unicode ranges.
 * Covers Emoticons, Misc Symbols, Transport, Flags, Supplemental, and more.
 */
const EMOJI_RANGES = [
  [0x1f600, 0x1f64f], // Emoticons
  [0x1f300, 0x1f5ff], // Misc Symbols and Pictographs
  [0x1f680, 0x1f6ff], // Transport and Map
  [0x1f1e0, 0x1f1ff], // Flags (regional indicators)
  [0x2600, 0x26ff],   // Miscellaneous Symbols
  [0x2700, 0x27bf],   // Dingbats
  [0x1f900, 0x1f9ff], // Supplemental Symbols and Pictographs
  [0x1fa00, 0x1fa6f], // Chess Symbols
  [0x1fa70, 0x1faff], // Symbols and Pictographs Extended-A
  [0xfe00, 0xfe0f],   // Variation Selectors
  [0x1f018, 0x1f0f5], // Playing Cards
  [0x1f200, 0x1f251], // Enclosed Ideographic Supplement
  [0x1f7e0, 0x1f7eb], // Geometric Shapes Extended
] as const

/**
 * Decorative / ornamental symbols to strip for cleaner TTS output.
 * These are visual-only glyphs that shouldn't be spoken.
 */
const DECORATIVE_SYMBOLS = /[※★☆◆◇◎●○□■△▲▽▼♡♥♦♣♠♪♫↗↘↙↖⤴⤵↔→←↑↓➡➤➜✔✘✖◆◇◎●○□■△▲▽▼]+/g

/**
 * Zero-width joiner (used in emoji sequences like 👨‍👩‍👧).
 */
const ZERO_WIDTH_JOINER = /\u{200D}/gu

/**
 * Combining enclosing keycap (e.g. 1⃣, 3⃣).
 */
const COMBINING_KEYCAP = /\u{20E3}/gu

// ────────────────────────────────────────────────────────────────────────────
// stripMarkdown
// ────────────────────────────────────────────────────────────────────────────

/**
 * Remove CommonMark / GitHub-Flavored Markdown syntax from raw text.
 *
 * Strips formatting so TTS reads only the natural-language content.
 * Runs **before** markdown rendering — HTML added by markdown-it is
 * intentionally NOT handled here.
 *
 * Transformations applied (order matters):
 *
 * 1. `![alt](url)` → `` (images removed entirely)
 * 2. `[text](url)` → `text` (links replaced with link text)
 * 3. `**bold**` → `bold`
 * 4. `__bold__` → `bold`
 * 5. `*italic*` → `italic` (single `*`, not `**`)
 * 6. `_italic_` → `italic` (single `_`, not `__`)
 * 7. `` `code` `` → `code` (inline code)
 * 8. `~~strike~~` → `strike`
 * 9. `# headings` → `headings` (ATX headings, any level)
 * 10. `> blockquote` → text (blockquote prefix)
 * 11. `- list item` / `* list item` → `list item` (unordered list)
 * 12. `1. list item` → `list item` (ordered list)
 * 13. `---` / `***` / `___` → `` (horizontal rules)
 *
 * @param text - Raw text potentially containing markdown syntax.
 * @returns Plain text with markdown syntax removed.
 *
 * @example
 * ```ts
 * stripMarkdown('**Hello** *world* [link](url)')
 * // => 'Hello world link'
 * ```
 */
export function stripMarkdown(text: string): string {
  let result = text

  // 1. Images: ![alt](url) → removed entirely
  result = result.replace(/!\[.*?\]\(.*?\)/g, '')

  // 2. Links: [text](url) → text
  result = result.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')

  // 3. Bold with **: **text** → text
  result = result.replace(/\*\*(.+?)\*\*/g, '$1')

  // 4. Bold with __: __text__ → text
  result = result.replace(/__(.+?)__/g, '$1')

  // 5. Italic with * (single, not **): *text* → text
  //    Must handle after bold removal to avoid matching leftover single * from **
  result = result.replace(/(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)/g, '$1')

  // 6. Italic with _ (single, not __): _text_ → text
  result = result.replace(/(?<!_)_(?!_)(.+?)(?<!_)_(?!_)/g, '$1')

  // 7. Inline code: `code` → code
  result = result.replace(/`(.+?)`/g, '$1')

  // 8. Strikethrough: ~~text~~ → text
  result = result.replace(/~~(.+?)~~/g, '$1')

  // 9. ATX headings: # (up to 6) → remove heading markers
  result = result.replace(/^#{1,6}\s+/gm, '')

  // 10. Blockquote: > prefix → remove
  result = result.replace(/^>\s?/gm, '')

  // 11. Unordered list markers: - / * / + → remove
  result = result.replace(/^[\t ]*[-*+]\s+/gm, '')

  // 12. Ordered list markers: 1. 2. → remove
  result = result.replace(/^[\t ]*\d+\.\s+/gm, '')

  // 13. Horizontal rules: ---, ***, ___ (on their own line) → remove
  result = result.replace(/^[-*_]{3,}\s*$/gm, '')

  return result
}

// ────────────────────────────────────────────────────────────────────────────
// filterPunctuation
// ────────────────────────────────────────────────────────────────────────────

/**
 * Remove punctuation and symbols that TTS engines don't need.
 *
 * **Keeps** sentence-ending punctuation (。！？.!?) because
 * {@link splitIntoSentences} relies on it for segmentation.
 *
 * Removals applied:
 * - Emoji characters (smileys, pictographs, flags, etc.)
 * - Decorative / ornamental symbols (★, ◆, ♪, etc.)
 * - Zero-width joiners and variation selectors
 * - Excessive consecutive punctuation (`!!!` → `!`, `。。。` → `。`, etc.)
 *
 * @param text - Text to clean of unnecessary punctuation.
 * @returns Text with decorative and excessive punctuation removed.
 *
 * @example
 * ```ts
 * filterPunctuation('Hello!!! 😊🎉 ★ World。。。')
 * // => 'Hello!  World。'
 * ```
 */
export function filterPunctuation(text: string): string {
  let result = text

  // Remove emoji characters by Unicode range
  for (const [low, high] of EMOJI_RANGES) {
    result = result.replace(buildUnicodeRangeRegex(low, high), '')
  }

  // Remove zero-width joiners and combining keycaps
  result = result.replace(ZERO_WIDTH_JOINER, '')
  result = result.replace(COMBINING_KEYCAP, '')

  // Remove decorative / ornamental symbols
  result = result.replace(DECORATIVE_SYMBOLS, '')

  // Reduce excessive exclamation marks: !!!、！！！→ !
  result = result.replace(/([！!]){2,}/g, '$1')

  // Reduce excessive question marks: ???、？？？→ ?
  result = result.replace(/([？?]){2,}/g, '$1')

  // Reduce excessive Chinese periods: 。。。→ 。
  result = result.replace(/([。]){2,}/g, '$1')

  // Reduce excessive English periods: ... → .
  result = result.replace(/\.{2,}/g, '.')

  // Reduce excessive semicolons: ;;; → ;
  result = result.replace(/([；;]){2,}/g, '$1')

  return result
}

// ────────────────────────────────────────────────────────────────────────────
// splitIntoSentences
// ────────────────────────────────────────────────────────────────────────────

/**
 * Split text into sentence-level segments for sequential TTS playback.
 *
 * Splitting strategy (applied in order):
 *
 * 1. **By sentence-ending punctuation** — Chinese `。！？；` and English
 *    `. ! ? ;` followed by whitespace or end-of-string.
 * 2. **By paragraph breaks** — `\n\n` (two or more consecutive newlines).
 * 3. **By maxLength** — If a segment exceeds `maxLength` (default 200 chars),
 *    recursively split on commas (`,，`), then on whitespace.
 *
 * Every segment's `text` is trimmed. Empty segments are filtered out.
 * The last segment has `isLast: true`.
 *
 * @param text - Text to split into TTS-ready segments.
 * @param maxLength - Maximum characters per segment before forced splitting.
 *   Defaults to 200.
 * @returns Array of {@link TextSegment} with sequential indices.
 *
 * @example
 * ```ts
 * splitIntoSentences('Hello world. How are you? I am fine.')
 * // => [
 * //   { text: 'Hello world.', index: 0, isLast: false },
 * //   { text: 'How are you?', index: 1, isLast: false },
 * //   { text: 'I am fine.', index: 2, isLast: true },
 * // ]
 * ```
 */
export function splitIntoSentences(
  text: string,
  maxLength = 200,
): TextSegment[] {
  // ── Stage 1: Split by sentence-ending punctuation ────────────
  const sentenceBreakPattern = /([^。！？；.!?;]+[。！？；.!?;])\s*/g
  const sentences: string[] = []
  let lastIndex = 0

  let match: RegExpExecArray | null
  while ((match = sentenceBreakPattern.exec(text)) !== null) {
    sentences.push(match[1].trim())
    lastIndex = match.index + match[0].length
  }

  // Any remaining text after the last punctuation
  const remaining = text.slice(lastIndex).trim()
  if (remaining.length > 0) {
    sentences.push(remaining)
  }

  // ── Stage 2: Split by paragraph breaks ──────────────────────
  const allSegments: string[] = []
  for (const sentence of sentences) {
    const parts = sentence.split(/\n\s*\n/)
    for (const part of parts) {
      const trimmed = part.trim()
      if (trimmed.length > 0) {
        allSegments.push(trimmed)
      }
    }
  }

  // ── Stage 3: Handle maxLength by recursive splitting ────────
  const segments: TextSegment[] = []
  let segmentIndex = 0

  /**
   * Recursively split a text segment that exceeds maxLength.
   * Tries comma-splitting first, then whitespace-splitting,
   * then falls back to hard character-length cuts.
   */
  function processSegment(segmentText: string): void {
    if (segmentText.length <= maxLength) {
      segments.push({
        text: segmentText,
        index: segmentIndex++,
        isLast: false,
      })
      return
    }

    // Try splitting by commas (Chinese + English)
    const commaParts = segmentText.split(/[,，]/)
    if (commaParts.length > 1) {
      const merged = mergeSmallParts(commaParts, '，', maxLength)
      for (const part of merged) {
        processSegment(part)
      }
      return
    }

    // Try splitting by whitespace
    const wordParts = segmentText.split(/\s+/)
    if (wordParts.length > 1) {
      const merged = mergeSmallParts(wordParts, ' ', maxLength)
      for (const part of merged) {
        processSegment(part)
      }
      return
    }

    // Fallback: hard cut at maxLength boundaries
    for (let i = 0; i < segmentText.length; i += maxLength) {
      segments.push({
        text: segmentText.slice(i, i + maxLength),
        index: segmentIndex++,
        isLast: false,
      })
    }
  }

  for (const segment of allSegments) {
    processSegment(segment)
  }

  // Mark the final segment
  if (segments.length > 0) {
    segments[segments.length - 1].isLast = true
  }

  return segments
}

// ────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ────────────────────────────────────────────────────────────────────────────

/**
 * Build a regex that matches a single character in the given Unicode range.
 */
function buildUnicodeRangeRegex(low: number, high: number): RegExp {
  return new RegExp(`[\\u{${low.toString(16)}}-\\u{${high.toString(16)}}]`, 'gu')
}

/**
 * Merge small text parts into segments that stay within maxLength.
 *
 * Greedily packs consecutive parts together until adding the next part
 * would exceed maxLength, then starts a new segment.
 *
 * @param parts - Array of trimmed text parts (e.g. comma-split fragments).
 * @param joiner - Separator string used when joining (e.g. '，' or ' ').
 * @param maxLength - Maximum characters per merged segment.
 * @returns Merged segments, each ≤ maxLength.
 */
function mergeSmallParts(
  parts: string[],
  joiner: string,
  maxLength: number,
): string[] {
  const result: string[] = []
  let current = ''

  for (const part of parts) {
    const trimmed = part.trim()
    if (trimmed.length === 0) continue

    const candidate =
      current.length === 0
        ? trimmed
        : current + joiner + trimmed

    if (candidate.length > maxLength && current.length > 0) {
      // Push current and start a new segment
      result.push(current)
      current = trimmed
    } else {
      current = candidate
    }
  }

  if (current.length > 0) {
    result.push(current)
  }

  return result
}
