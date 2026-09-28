# YAML font format, version 1

Both embedded and external fonts are UTF-8 YAML documents. Unknown fields, duplicate mapping keys, YAML tags, invalid types, unsupported format versions, invalid Unicode keys, control characters in artwork, and invalid dimensions are errors. YAML parsing checks syntax; explicit semantic validation checks the font contract.

Keys are case-sensitive. Row and baseline indexes are **zero-based**. Glyph keys are unquoted integer Unicode scalar values, normally written in decimal; the sole named key is the required `default` glyph. Legacy CP437 character numbers are not Unicode keys. For example, `199` represents Ç, `9472` represents ─, and `128512` represents 😀.

```yaml
format_version: 1
font:
  id: example
  name: Example
  author: Your name          # Optional
  license: MIT              # Optional descriptive metadata
  description: My font      # Optional; one line
metrics:
  height: 5
  baseline: 3
  spacing: 1                # Internal spacer width; optional, default 1
  space_width: 3            # Optional, default 3
  separator: ''             # Optional, appended after every glyph on every row

decorations:                # Optional
  border:                  # Optional
    top: '░'
    bottom: '░'
    height: 1              # Optional, default 1; thickness on each enabled side
  underline:               # Optional; underline has no effect if absent
    row: 4
    char: '▄'

glyphs:
  default:
    rows: [' ▄ ', '▄ ▀', ' ▄ ', '   ', ' ▀ ']
  32: # SPACE; optional, synthesized from space_width if absent
    width: 3
    rows: []
  65: # A
    rows:
      - '   '
      - '▄▀▄'
      - '█▀█'
      - '█ █'
      - '   '
  103: # g
    rows:
      - '   '
      - '   '
      - '▄▀█'
      - '▀▀█'
      - '▀▀ '
    underline:
      rows:
        4: '▀▀▄'
```

## Glyph sizing

The optional `metrics.separator` is literal artwork appended after each glyph, distinct from blank internal spacers; compact mode leaves it intact.

A glyph has `rows`, optional `width`, optional `underline`, and optional `borders`. Width is inferred from the widest normal row unless explicitly set. Measurements are terminal columns, not UTF-8 bytes. Short rows are padded on the right with spaces; missing rows are padded at the bottom. Leading blank rows must be included explicitly. More rows than `metrics.height`, or artwork wider than the declared/inferred width, are errors.

A glyph with no artwork requires a positive explicit width, except SPACE, which can use `space_width`. If SPACE is omitted, a blank glyph is synthesized; whitespace never becomes the unknown-character symbol. All other missing code points use `default` without borrowing artwork from another font.

Use quoted strings to protect spaces and YAML-sensitive characters. Single-quoted YAML preserves backslashes literally (write an apostrophe as two apostrophes). Double-quoted YAML uses YAML's own escape syntax. CLI text escape processing never runs on font artwork.

## Underline behavior and overrides

When enabled, the default algorithm replaces ASCII spaces in the font's designated underline row with its underline character, preserving non-space artwork. Padding is included. Underline never adds an extra row; `height` must already reserve its space.

Glyph overrides replace whole specified rows **after** the generic underline operation. Replacement rows are padded to the same glyph width. They may replace any row inside the font, not just the default underline row.

```yaml
underline:
  enabled: true            # Default true
  spacer: false            # Default true; suppress underline in owned spacers/padding
  rows:
    3: 'custom'
    4: 'rows  '
```

Replacement artwork requires sufficient glyph width. To preserve a descender row exactly, supply that exact row as an override. To suppress the underline operation on the entire glyph, use `underline: {enabled: false}`. Disabled underlining cannot also specify replacement rows. `spacer: false` suppresses underline in the spacer following this glyph and in outer padding owned by this glyph. It does not disable the glyph's own underline. Leading padding belongs to the first character; trailing padding belongs to the last. Both are always one column on nonempty lines. `--compact` removes internal spacers but never these outer columns.

A glyph override requires font-level underline metadata. Fonts align by baseline when mixed; each keeps its own underline row relative to its baseline. There is no cross-font underline alignment or rewriting.

## Border artwork overrides

Borders are globally enabled/disabled by the invocation. Ordinary glyphs receive a repeated border character across their complete cell width. Specialized glyphs may override these rows, including with spaces:

```yaml
borders:
  top: '   █   '
  bottom: '       '
  spacer: false
```

Missing top/bottom fields use the owning font's border defaults (or the global character override). `spacer: false` leaves this glyph's following border spacer and owned outer padding blank. These artwork rows define the shape and repeat for each row of border thickness. Explicit top/bottom character overrides replace their non-space ink while preserving spaces and width. Extra monospace padding beside a custom border is blank.

The C# rounded corner/line-end symbols occupied border rows directly. Their inner artwork is now height-bounded and their outer artwork is stored here, so they can render with either, both, or neither border enabled without indexing outside an array.

## Border endcaps

`decorations.border.endcaps` is optional. When supplied, both `left` and `right` definitions are required. Each has a top sequence, one repeatable middle row, a bottom sequence, and an optional explicit width:

```yaml
decorations:
  border:
    top: '▀'
    bottom: '▄'
    height: 1
    endcaps:
      left:
        width: 4                  # Optional; inferred from the widest row
        top: ['  █▀', ' █▀ ', '█▀  ']
        middle: '█   '
        bottom: ['█▄  ', ' █▄ ', '  █▄']
      right:
        top: ['▀█  ', ' ▀█ ', '  ▀█']
        middle: '   █'
        bottom: ['  ▄█', ' ▄█ ', '▄█  ']
```

All sequences are written in output order, from top to bottom. The top sequence includes the row that joins the horizontal top border; the bottom sequence includes its bottom joining row. Endcaps sit outside the line's existing one-column outer padding. Wrappers sit outside the complete frame.

Top-only output uses exactly the top sequence, anchored to the first row. Bottom-only output uses exactly the bottom sequence, anchored to the final row. All unused side rows are spaces of the same width. **Neither single-border mode uses the middle row.** With both borders enabled, the middle row fills the gap between the two sequences. With neither border enabled, there is no endcap artwork or reserved side width, even if `--border-endcaps` is set.

This lets font authors choose precisely how far a single cap extends. Top/bottom arrays must each contain 1–128 rows; use space-only rows for deliberately blank artwork. The `middle` string is required and may also be blank. Each side has a width of 1–64 terminal columns, inferred across all its rows unless set explicitly. Short rows are padded on the right; oversized rows and control characters are errors. Left and right widths may differ.

Mixed-font output uses the first character's font for the left and the last character's font for the right, including their colors. Blank logical lines use the style active at the newline; interline gaps remain unframed. The complete line reserves room for the largest enabled top section and largest enabled bottom section on either side. If they do not fit, blank rows are added around the original glyph body, with the extra row above when the number is odd. Glyphs remain baseline-aligned and keep their own underline positions.

For border thickness above one row, the first top endcap row repeats for every top border row, followed by the rest of the top sequence. The last bottom endcap row likewise repeats for every bottom border row. The corner curves themselves are not stretched.

Explicit `--border-char` replaces non-space ink on top, bottom, left, and right. `--endcap-char` affects both sides; `--left-endcap-char` and `--right-endcap-char` affect one side. The horizontal-only options do not affect endcap corners. Replacement is by occupied terminal cells, preserving spaces and the width of wide/combining artwork. These changes never rewrite the text glyphs or underlines.

Older fonts with no endcap definitions remain valid. When endcaps are requested they use a one-column fallback: top and bottom rows use their matching font border characters, and the middle uses the top border character. Only the enabled corner rows appear for single-border output. Fonts with no border metadata use `░` for that fallback.

## Baseline and line layout

`baseline` identifies the shared alignment row. A line's ascent is the maximum baseline among its fonts; its descent is the maximum `height - baseline - 1`. Its body height is ascent + 1 + descent. This may be taller than any individual font when their baselines differ substantially. Top/bottom border rows are outside this body.

Each glyph supplies its font's default border characters. Internal spacers inherit the preceding character's defaults; the first and last characters supply the outer padding defaults. The largest configured border height among fonts on the line supplies thickness. A font without border metadata uses `░` and height 1 when borders are requested. Global height overrides replace the default thickness. Character overrides replace non-space ink in defaults and explicit glyph border artwork, preserving all ASCII spaces and terminal width.

## Validation and limits

- `format_version` must be 1. `font.id` and `font.name` must be nonempty.
- Font metadata is single-line text without control characters.
- Height: 1–128; baseline and underline row must lie within height.
- Glyph width and space width: 1–512 terminal columns.
- Spacing: 0–128; border height: 1–16.
- Endcap width: 1–64; top and bottom sections: 1–128 rows each. Both sides and their middle rows are required when endcaps are defined.
- Border/underline characters must occupy exactly one terminal column and contain no controls.
- Separator: at most 16 terminal columns, without controls.
- Glyph keys must be Unicode scalar values, excluding control characters and surrogate code points. Integer-looking strings are rejected.
- Glyph rows cannot contain controls, tabs, newlines, or ANSI escape sequences.
- `default` is mandatory. Duplicate keys anywhere in the document are rejected.
- Font file size: 4 MiB. Each text file/stdin input: 16 MiB. Rendered input: 100,000 scalar values including expanded tabs and line breaks. Output: 64 MiB total and approximately 1 MiB per row, including color escapes.

These limits turn malformed or excessive input into diagnostics instead of silent truncation. Files contain only data; external paths, imports, expressions, and scripts have no role in the schema.
