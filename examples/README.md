# Small example fonts

These files demonstrate complete, editable font definitions without including the full alphabet or every built-in glyph. Both include author/license/description metadata, height and baseline, spacing and space width, a default glyph, underline configuration, and explicit left/right endcaps.

| File | Artwork | Height | Shared glyphs | Additional glyphs |
| --- | --- | --- | --- | --- |
| [micro.yaml](micro.yaml) | Compact ASCII shapes with rectangular borders | 3 | 46 + default | 😀 (a Unicode key with two-column artwork) |
| [ornate-frame.yaml](ornate-frame.yaml) | Original Ultra glyphs and fancy block-art borders | 7 | 46 + default | None |

The shared repertoire covers all of these samples:

```text
abcABCxyzXYZZ
0123456789
bigtext BIGTEXT
!?., -_:/
jpq mMwW
```

Exact supported letters:

- Uppercase: `ABCEGIMTWXYZ`
- Lowercase: `abcegijmpqtwxyz`
- Digits: `0123456789`
- Punctuation: `!?.,-_:/`
- Space

`g`, `j`, `p`, `q`, and `y` demonstrate descenders and underlining. `m`, `M`, `w`, and `W` demonstrate wider glyphs. Space, hyphen, underscore, colon, and slash are useful for sample headings and labels. The repeated `Z` in the first sample is intentional; it reuses the same glyph.

The required `default` glyph handles anything outside the repertoire. A normal word using other letters will therefore contain fallback symbols; use a built-in or export one if you need a complete alphabet.

## Preview and validate

From the project root:

```sh
cargo run -- --check-font examples/micro.yaml
cargo run -- --check-font examples/ornate-frame.yaml
cargo run -- --font-file examples/micro.yaml --border --border-endcaps \
  'abcABCxyzXYZZ\n0123456789\nbigtext BIGTEXT\n!?., -_:/\njpq mMwW'
cargo run -- --font-file examples/ornate-frame.yaml --border --border-endcaps \
  --underline 'abcABCxyzXYZZ\n0123456789\nbigtext BIGTEXT\n!?., -_:/\njpq mMwW'
```

Toggle `--compact`, `--underline`, and top/bottom borders to explore how the artwork composes. Micro's third row contains both letter artwork and underline space; its explicit `A`, `g`, `p`, and `q` underline variants illustrate that distinction. Its remaining glyphs use the generic underline operation. Ornate preserves Ultra's original per-glyph underline variants.

`--demo` shows the full alphabet and punctuation, so it will intentionally display fallback glyphs for characters these small examples omit. Use the sample strings above for a preview containing only supported characters.

The example-font tests discover every `.yaml`/`.yml` font in this directory and check the shared repertoire, metadata, and rendering modes. See [the font format](../docs/font-format.md) when adding another example.
