# bigtext

Compose large Unicode text with editable YAML fonts and formatting that changes as you move through the command line. The project directory is `bigtext2`; the Rust package and executable are **bigtext**.


```text

    █▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀█
   █▀                                                    ▀█
  █▀   ██      ██           ██                     ██     ▀█
  █    ██▀▀▀█▄ ▄▄ ▄█▀▀▀█▄ ▀▀██▀▀ ▄█▀▀▀█▄ ▀█▄ ▄█▀ ▀▀██▀▀    █
  █    ██   ██ ██ ██   ██   ██   ██▄▄▄██   ███     ██      █
  █    ██   ██ ██ ██   ██   ██   ██       ██ ██    ██      █
  █▄   ██████▀ ██ ▀██████   ██   ▀█████▀ ██   ██   ██     ▄█
   █▄              ▄▄▄▄█▀                                ▄█
    █▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄█

```

Basic, Tall, and Ultra are embedded in the executable. No font directory, C# runtime, or network connection is needed to run it. Each font contains 257 Unicode glyphs plus a fallback, adapted from the original C# artwork with separate underline variants.

## Build and run

Use a current stable Rust toolchain (edition 2024; Rust 1.88 or later).

```sh
cargo build --release
cargo run --release -- --help
cargo run --release -- --tall --demo
```

To install explicitly when ready, run `cargo install --path . --locked`. This project does not install or replace an existing `bigtext` automatically. Cargo may place build artifacts outside this directory if your user configuration sets `target-dir`.

## Formatting in order

Text arguments concatenate exactly, with **no added spaces**. Put intentional spaces inside quotes. Options affect subsequent text until changed; switching fonts preserves underline, spacing, compact mode, color, and literal mode.

```sh
bigtext --basic 'Hello ' --color cyan --underline --ultra 'World' \
  --no-color --no-underline --tall '!'
bigtext --narrow 'TIGHT' --spacing 2 ' LOOSE' --wide ' NORMAL'
bigtext --spacing 2 --compact 'TIGHT' --no-compact ' SPACED'
bigtext --monospace 'WiWi' --no-monospace ' WiWi'
bigtext --literal 'C:\work\file' --reset-style ' back to defaults'
bigtext -- --this-is-text
```

`--reset-style` restores Basic, normal spacing, compact mode off, proportional width, no underline/color, and escape decoding. It does not reset global output settings.

Mixed fonts align by baseline. Each logical line reserves the largest ascent and descent required by its fonts. **Underlines remain in each font's own row**, including their descender artwork; they are not shifted to line up.

Every nonempty logical line has exactly one outer padding column at each end. Leading padding uses the first character's decoration and color; trailing padding uses the last character's. Internal spacers are separate from the glyphs and use the **preceding character's** spacing, underline position, border behavior, and color. They follow the preceding character even across argument and font boundaries.

`--compact` suppresses internal spacers after subsequent characters; `--no-compact` restores their configured spacing. The final character before a style change owns that boundary's spacer. Compact mode preserves outer padding, actual input spaces, glyph artwork, and monospace cell padding. `--spacing N` configures only internal spacing; `--narrow` is `--spacing 0`, and `--wide` restores font-default spacing. Changing the spacing setting does not turn compact mode off. `--reset-style` does.

Monospace uses the selected font's widest glyph (including fallback) unless `--monospace-width N` sets a cell width; a cell too small for an actual glyph is an error rather than clipped output.

## Borders and wrappers

Borders are global: the final settings apply to every logical line, including text before those options. `--border` does not consume the next argument. Setting a character does not implicitly enable borders.

```sh
bigtext --border 'Hello'                         # Both default borders
bigtext --border --border-char '=' 'Hello'      # Both use =
bigtext --top-border --top-border-char '-' 'Hi' # Only top border
bigtext --border --border-height 2 'Thick'
bigtext --slash-comment --border 'Section'
bigtext --prefix '/* ' --suffix ' */' 'Section'
```

By default, each glyph uses its own font's border characters, and spacers/padding use their owning character's defaults. Each line reserves the maximum border height required by its fonts (one row if unspecified). `--border-height` overrides the height. Character overrides replace non-space border artwork, including glyph-specific border rows, while preserving its spaces and width. Explicit glyph exceptions can still leave gaps in borders.

`--no-border`, `--no-top-border`, and `--no-bottom-border` switch borders off globally. `--border-top` and `--border-bottom` are aliases. Wrappers apply to every emitted row, including interline gaps. Available shortcuts include `--slash-comment`, `--hash-comment`, `--tick-comment`, `--lua-comment`, `--block-comment`, and `--html-comment`; `--no-comment` clears both wrappers. Prefixes/suffixes are literal and are not escape-decoded.

## Combining borders across lines

`--combine-borders` puts all logical lines inside one bordered block. Separate
frames remain the default; `--no-combine-borders` restores that behavior. These
options are global and the last setting wins. They do not enable borders.

```sh
bigtext --ultra -b --border-endcaps --combine-borders --line-spacing 0 'bigtext\nrocks!'
```

The widest rendered line sets the interior width. Shorter lines stay left-aligned
and receive plain spaces on the right; their underlines do not extend into that
padding. Each line retains its own fonts, baseline, compact setting, and colors.
`--line-spacing` adds blank rows **inside** the frame (default 1); use 0 for the
example above. Blank rows in the font artwork remain intact.

The first logical line supplies the top border artwork and thickness; the last
supplies the bottom. When a boundary line is shorter, its last character's font
extends that horizontal border (an empty line uses its active font). The first
and last actual characters across the block supply the left and right endcaps
and their colors. Character overrides still apply to their respective surfaces.
Top-only and bottom-only modes draw only their outer corners, without repeating
middle side artwork. Prefixes and suffixes wrap every final row outside the
frame, including gap rows. With no borders, or only one logical line, rendering
is unchanged.

## Border endcaps

`--border-endcaps` enables font-defined sides outside the existing one-column padding; `--no-border-endcaps` disables them. Both are global options. Endcaps do not implicitly enable horizontal borders:

| Horizontal borders | Endcap artwork |
| --- | --- |
| Both | Top corners + repeated middle + bottom corners |
| Top only | Top corners only; blank sides below them |
| Bottom only | Bottom corners only; blank sides above them |
| Neither | No endcaps and no added side width |

Unused rows retain the side width, so the text stays aligned. The first character's font/color supplies the left side and the last character's supplies the right. Taller mixed-font lines repeat the middle only when both borders are enabled. Oversized corner sections add vertical space around the text, without clipping artwork. Compact mode preserves endcaps and outer padding.

```sh
bigtext --ultra -b --border-endcaps --block-comment bigtext
bigtext --ultra --top-border --border-endcaps bigtext
bigtext --ultra --bottom-border --border-endcaps bigtext
bigtext --ultra -b --border-endcaps --border-char '=' bigtext
bigtext --ultra -b --border-endcaps --top-border-char '-' \
  --bottom-border-char '=' --left-endcap-char '[' --right-endcap-char ']' bigtext
```

`--border-char CHAR` replaces ink on all four surfaces. `--top-border-char` and `--bottom-border-char` override the horizontal borders; `--endcap-char` overrides both sides; `--left-endcap-char` and `--right-endcap-char` override individual sides. Later settings win for the surfaces they affect. Corner artwork belongs to its endcap, so a top/bottom-specific override does not recolor those corners. Spaces are preserved, and wide artwork is replaced by the equivalent number of terminal cells. Text and underline artwork are unaffected.

Basic and Tall have rounded shaded endcaps. Ultra uses the fancy `█▀` / `█▄` frame. Fonts without endcap definitions get straight one-column sides; in top/bottom-only mode the fallback appears only on the enabled border's rows. Font authors can customize the corner depth, middle row, side width, and characters in YAML.

[examples/ornate-frame.yaml](examples/ornate-frame.yaml) is a small Ultra subset covering the shared example character set, with the `█▀` / `█▄` frame:

```sh
cargo run -- --font-file examples/ornate-frame.yaml -b --border-endcaps bigtext
```

The full Ultra font already uses this design; the example is a smaller reference for editing your own frame. See the [endcap schema](docs/font-format.md#border-endcaps).

## Input, whitespace, and Unicode

```sh
printf 'Hello\nWorld\n' | bigtext --tall
bigtext --underline --text-file message.txt
printf World | bigtext 'Hello ' --tall --stdin
bigtext --tall '\u{2500}'
bigtext 'First\n\nThird'
bigtext First --newline --tall Second
```

With no text sources, redirected stdin is read using the final style. An interactive invocation without sources shows help. Explicit sources (`TEXT`, `--text-file`, `--stdin`, `--demo`, `--newline`) are inserted in order. Explicit text does not implicitly append piped input. Stdin can be inserted once.

Input files and stdin must be UTF-8. Unix newlines, Windows CRLF, and standalone CR become logical line breaks, even across input-source boundaries. A final newline terminates the last line; it does not add a phantom banner. Consecutive newlines preserve blank logical lines, using the style active at each newline. Empty input emits nothing. Blank logical lines reserve the active font's height.

Tabs advance to `--tab-width N` input-character stops (default 4). These count Unicode scalar values, not rendered banner columns. `--line-spacing N` sets blank output rows between logical lines (default 1). Leading/trailing spaces are preserved.

Supported escapes: `\n`, `\r`, `\t`, `\\`, `\u0123`, `\u{123}`, and `\U0001F600`. Braced escapes accept 1–6 hexadecimal digits; unbraced forms require exactly 4 or 8. Invalid escapes, surrogates, and out-of-range code points are errors. Other control characters are rejected. `--literal` disables escape processing for subsequent text; `--no-literal` restores it. Actual newline/tab characters still work in literal mode.

Decoded Unicode characters are looked up in the selected font. Missing characters use that font's required `default` glyph. Unicode normalization and multi-code-point ligatures are not performed: a combining mark is its own lookup, and a multi-code-point emoji sequence uses multiple lookups.

## Fonts

```sh
bigtext --list-fonts
bigtext --font-info tall
bigtext --export-font tall > my-font.yaml
bigtext --check-font my-font.yaml
bigtext --font-file my-font.yaml 'Hello'
bigtext --font-info my-font.yaml
```

External fonts use exactly the same loader and format as embedded fonts. Any filename extension is accepted. `--font NAME` selects an embedded font; `--font-file PATH` explicitly selects a file. `--basic`, `--tall`, and `--ultra` are shortcuts for built-ins. `--demo` inserts the selected font's letters, numbers, punctuation, and extended glyphs, under the current style.

Start with [examples/micro.yaml](examples/micro.yaml) or export a built-in. Both example fonts cover `abcABCxyzXYZZ`, `0123456789`, `bigtext`, `BIGTEXT`, and `!?.,`, plus spaces, `-_:/`, descenders `jpq`, and wide letters `mMwW`. See the [example font guide](examples/README.md) for their exact repertoire and preview commands. See [the font format](docs/font-format.md) for the complete schema and [the design specification](docs/design.md) for behavior and architecture.

## Output and diagnostics

The 16 console colors accept names such as `red`, `dark-blue`, `gray`, and `white` (case-insensitive, with optional hyphens/underscores). `--color-mode auto` is the default and enables ANSI color only on terminal stdout, respecting `NO_COLOR` and `TERM=dumb`. Use `always` or `never` to override. Color is span-specific; wrappers and interline gaps are uncolored.

Errors and `--verbose` diagnostics go to stderr. All input and rendering are validated before the banner is written. Errors exit with status 2; success and a closed downstream pipe exit with status 0. `--pause` waits for a key using stderr and requires interactive stdin and stderr.

The supported output is Unicode terminal text, not only ASCII. Display widths use the Rust [unicode-width](https://docs.rs/unicode-width/latest/unicode_width/) library; actual terminal glyph appearance and ambiguous-width policy depend on the terminal font.

## Development and verification

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Tests compare every imported glyph in both normal and underlined modes against independently captured C# output. They also cover mixed baselines, font-specific underline positions, borders, spacing, Unicode mapping, malformed fonts, ordered options, whitespace, escapes, file/stdin input, export/reload, and redirected color behavior.

Font parsing uses [serde_yaml_ng](https://docs.rs/serde_yaml_ng/latest/serde_yaml_ng/). Files are data-only: no scripts, executable expressions, custom tags, or implicit file loading. See the format documentation for validation rules and resource limits.

The original C# and Rust projects remain separate. [tools/import_csharp.py](tools/import_csharp.py) records the artwork migration procedure; only regeneration requires .NET 10 and Python. Ordinary builds and execution use Rust alone.

Wrapping, centering, gradients, animation, and an inline markup language are deferred. Formatting is expressed through ordered command-line options; text escapes only insert characters and line breaks.

## Examples

```sh
λ cargo run -- --border --border-char '=' --basic 'hello ' --underline --tall W --no-underline 'orld!'
=============================================
                                           ▄
                      █ █ █         █    █ █
 █    ▄  █  █         █ █ █ ▄▀▄ █▀▄ █  ▄▀█ █
 █▀▄ █▄█ █  █  ▄▀▄    █ █ █ █ █ █   █  █ █ █
 █ █ ▀▄▄ █▄ █▄ ▀▄▀    ▀▄▀▄▀ ▀▄▀ █   █▄ ▀▄█ ▄
                      ▄▄▄▄▄▄
=============================================

λ bigtext --ultra -b "bigtext" --prefix "/* " --suffix " */"
/* ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀ */
/*                                                    */
/*  ██      ██           ██                     ██    */
/*  ██▀▀▀█▄ ▄▄ ▄█▀▀▀█▄ ▀▀██▀▀ ▄█▀▀▀█▄ ▀█▄ ▄█▀ ▀▀██▀▀  */
/*  ██   ██ ██ ██   ██   ██   ██▄▄▄██   ███     ██    */
/*  ██   ██ ██ ██   ██   ██   ██       ██ ██    ██    */
/*  ██████▀ ██ ▀██████   ██   ▀█████▀ ██   ██   ██    */
/*              ▄▄▄▄█▀                                */
/* ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄ */

λ bigtext --font ultra -b "bigtext\nrocks!" --border-endcaps --combine-borders
  █▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀█
 █▀                                                    ▀█
█▀   ██      ██           ██                     ██     ▀█
█    ██▀▀▀█▄ ▄▄ ▄█▀▀▀█▄ ▀▀██▀▀ ▄█▀▀▀█▄ ▀█▄ ▄█▀ ▀▀██▀▀    █
█    ██   ██ ██ ██   ██   ██   ██▄▄▄██   ███     ██      █
█    ██   ██ ██ ██   ██   ██   ██       ██ ██    ██      █
█    ██████▀ ██ ▀██████   ██   ▀█████▀ ██   ██   ██      █
█                ▄▄▄▄█▀                                  █
█                                          ▄▄            █
█                          ██              ██            █
█    ▄█▀▀▀ ▄█▀▀▀█▄ ▄█▀▀▀▄█ ██  ▄▀  ▄█▀▀▀█▄ ██            █
█    ██    ██   ██ ██      ██▄█▄   ▀█▄▄▄▄  ██            █
█    ██    ██   ██ ██   ▄▄ ██  ██       ██               █
█▄   ██    ▀█████▀ ▀█████▀ ██   ██ ▀█████▀ ██           ▄█
 █▄                                                    ▄█
  █▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄█

```
