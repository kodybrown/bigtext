# bigtext design and implementation decisions

## Goal and scope

Create a fresh Rust project in `~/Projects/bigtext2/`, with package and binary name `bigtext`. Preserve the original C# fonts' dynamic composition and hand-drawn underline variants while replacing duplicated glyph-specific code with editable, embedded YAML definitions. Support the same format for external fonts at runtime.

The original `bigtext_cs` and `bigtext-rust` repositories are references, not implementation dependencies. Their current CLI/build defects are not compatibility requirements.

## Command model

An invocation is an ordered sequence of style changes and text sources. Each text source captures the current style; later style changes do not alter earlier text. Text sources concatenate without inferred spaces. Named built-in shortcuts are `--basic`, `--tall`, and `--ultra`; external fonts use `--font-file PATH`.

Span state consists of font, underline, spacing, compact mode, proportional/monospace mode, color, and escape interpretation. Switching font preserves other span state. Explicit negative options remove formatting. Reset restores all span defaults.

Borders, endcaps, prefix/suffix, color output policy, tab width, and interline spacing are invocation-wide. Last settings win across all text. `--border` is a boolean and never consumes text. `--border-char CHAR` sets characters separately without enabling borders. Independent top/bottom flags and character overrides are supported.

Short options are explicit aliases; clustered short options are not supported. `--option=value` is accepted for value-taking options. `--` ends option parsing but does not change escape interpretation. Utility commands are exclusive with text input. Help/version return immediately when encountered.

Input files and stdin capture the style at their insertion position. Without explicit sources, redirected stdin uses the final style. Interactive no-input invocations show help. Text decoding and font loading fail before stdout receives any banner.

## Font and layout model

Fonts contain metadata, height/baseline, spacing, Unicode-keyed glyphs, required fallback artwork, default decorations, and explicit glyph overrides. All font files are data-only. Normal and underlined artwork remain distinct where needed.

Glyph rows are normalized to their declared/inferred terminal width. Baselines align per logical line; maximum ascent and descent reserve space. Underlines stay attached to each font's own artwork, even when they do not align across fonts. Global borders add rows outside the combined line body.

Normal internal spacing is font-specific and belongs to the preceding character. Explicit spacing and compact mode persist across font switches. Each nonempty line has one leading and one trailing padding column, owned by its first and last characters. Spacer and padding decoration/color follow the owner, including glyph-specific suppression and font-specific underline position. Compact mode removes internal spacers only; toggling it off restores configured spacing. Narrow/zero spacing also preserves outer padding. Border defaults are per owning font, while enablement and explicit character/height overrides remain global. Monospace chooses a per-font maximum width including fallback, or an explicit cell width, with right padding. Too-small cells are errors. A separator may be specified in the font. Blank logical lines retain their active font's height. Logical lines have independent widths and heights.

## Architecture

- `font.rs`: embedded font registry, YAML loading, semantic validation, normalized glyph data, underline variant selection.
- `escape.rs`: strict character escape decoding; no formatting language.
- `cli.rs`: sequential option parsing and immutable style snapshots per input source.
- `render.rs`: pure composition into rows, baseline layout, borders, spacing, wrappers, and optional ANSI span colors.
- `main.rs`: filesystem/stdin input, terminal detection, stdout/stderr, utilities, error status, and optional pause.

Output is composed before writing, keeping errors out of partial banners. Resource limits bound input and output. The renderer is also exposed through the library target for reuse and testing.

## Artwork migration

Basic, Tall, and Ultra were evaluated through an isolated C# harness in normal and underlined modes. This avoids transliterating thousands of C# switch branches. Each imported font has 257 Unicode glyphs plus the default symbol.

Legacy numeric PC character mappings were translated to Unicode (including graphical control-code symbols and CP437 extended characters). Actual control characters are not glyphs. Empty/unimplemented C# cases remain absent and therefore receive fallback behavior.

C# row widths occasionally disagreed with their declared widths, notably in Ultra. The imported data pads rows to the widest normal/underlined artwork rather than clipping or leaving ragged output. Specialized corner/end symbols had body rows plus border-area artwork; this is split into body and border overrides to eliminate their no-border crashes. Outer padding is independent of the internal spacing width; narrow and compact modes retain one column at both ends.

`tools/import_csharp.py` and `tools/CSharpExport.cs` make the extraction reproducible. `tests/fixtures/csharp-glyphs.json` stores the separately captured expected normal and underlined rows. Normal Rust builds never execute the importer.

## Verification

The test suite compares all 774 imported definitions (including the three fallbacks), in both normal and underlined modes. Whole-repertoire rendering is checked across all combinations of underline, narrow spacing, and top/bottom borders. Additional tests exercise different baseline/underline positions, ordered style persistence/reset, Unicode escapes and fallback, whitespace/newlines, monospace, invalid schemas, duplicate keys, export/reload, actual CLI input sources, diagnostics, and color policy.

Linux builds and CLI behavior are verified locally. The implementation uses portable Rust/std APIs and the console crate; Windows terminal behavior still requires a Windows acceptance run.

## Deferred

Automatic wrapping, centering, gradients, animation, and text-embedded formatting markup. No font scripts or executable expressions. No implicit installation, replacement of an existing executable, publishing, or integration into the original repositories.

## Initial implementation verification — 2026-09-28

- `cargo test --locked`: 24 tests passed, including the full imported glyph comparison.
- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo build --release --locked`: passed on Linux.
- Release executable checked with a pseudo-terminal: no-input help, automatic color, `NO_COLOR`, and interactive keypress pause passed.
- Closed downstream stdout pipe exits successfully without an error diagnostic.

The project is initialized as a new Git repository. No initial commit, installation, publishing, or changes to the original project repositories were performed.

## Independent spacers and compact mode

Added ordered `--compact` / `--no-compact` controls. Composition is leading padding, glyph, independently styled internal spacer, next glyph, and trailing padding. Internal spacers inherit the preceding character, including across underline, font, and color changes. Glyph data and underline artwork are unchanged. Regression tests cover ownership, font border defaults and overrides, different underline baselines, compact toggles and reset, fixed outer columns, explicit spaces, monospace padding, and CLI input.

## Font-defined border endcaps

Endcaps are optional font data under `decorations.border.endcaps`. Each side has top rows, one middle row, and bottom rows. Both horizontal borders render all three sections; top-only renders only the top corner; bottom-only renders only the bottom corner; neither renders no sides and reserves no side width. Single-border modes never repeat the middle row. Unused side rows are blank but retain width.

The frame composes outside outer padding and inside prefix/suffix. The first/last character owns the left/right side, respectively. Mixed fonts, underlines, and compact spacing keep their existing text behavior. Corner sections can increase available vertical space without clipping or distorting glyphs. Border thickness repeats outer joining rows, and blank logical lines use their active font's frame.

Explicit border characters now act as ink replacement masks: all non-space terminal cells are replaced, including custom glyph border rows and endcaps. ASCII spaces and width remain intact. Side-specific overrides own their corner artwork; horizontal-specific overrides affect only their horizontal surfaces. Options remain global and last-wins per affected surface.

Basic and Tall use rounded shaded endcaps; Ultra uses the fancy block-art frame with `▀` above, `▄` below, and `█▀` / `█▄` endcaps. Their existing glyph and underline data is retained. Both examples contain a small shared repertoire for letter, digit, punctuation, descender, and width demonstrations, plus their fallback artwork. The ornate frame example retains the corresponding Ultra glyphs; Micro supplies its own three-row ASCII artwork and an additional Unicode sample. The C# glyph snapshot remains unchanged; endcap geometry and masking have their own regression tests.

Endcap verification: 43 tests passed, including the original glyph snapshot checks. Formatting, Clippy with warnings denied, and the Linux release build passed. Full, top-only, and bottom-only Ultra/ornate examples were rendered and inspected.
