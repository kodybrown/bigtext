use crate::Result;
use serde::Deserialize;
use serde_yaml_ng::{Mapping, Value};
use std::{collections::BTreeMap, fs, path::Path};
use unicode_width::UnicodeWidthStr;

pub const BUILTINS: &[(&str, &str)] = &[
    ("basic", include_str!("../fonts/basic.yaml")),
    ("tall", include_str!("../fonts/tall.yaml")),
    ("ultra", include_str!("../fonts/ultra.yaml")),
];
pub const MAX_FONT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub description: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    pub height: usize,
    pub baseline: usize,
    #[serde(default = "one")]
    pub spacing: usize,
    #[serde(default = "three")]
    pub space_width: usize,
    #[serde(default)]
    pub separator: String,
}
fn one() -> usize {
    1
}
fn three() -> usize {
    3
}
fn yes() -> bool {
    true
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Border {
    pub top: String,
    pub bottom: String,
    #[serde(default = "one")]
    pub height: usize,
    pub endcaps: Option<Endcaps>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endcaps {
    pub left: Endcap,
    pub right: Endcap,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endcap {
    pub width: Option<usize>,
    pub top: Vec<String>,
    pub middle: String,
    pub bottom: Vec<String>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Underline {
    pub row: usize,
    pub char: String,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decorations {
    pub border: Option<Border>,
    pub underline: Option<Underline>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlyphUnderline {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub spacer: bool,
    #[serde(default)]
    pub rows: BTreeMap<usize, String>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlyphBorders {
    pub top: Option<String>,
    pub bottom: Option<String>,
    #[serde(default = "yes")]
    pub spacer: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glyph {
    pub width: Option<usize>,
    pub rows: Vec<String>,
    pub underline: Option<GlyphUnderline>,
    pub borders: Option<GlyphBorders>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format_version: u32,
    font: Metadata,
    metrics: Metrics,
    #[serde(default)]
    decorations: Decorations,
    glyphs: Mapping,
}
#[derive(Clone, Debug)]
pub struct Font {
    pub meta: Metadata,
    pub metrics: Metrics,
    pub decorations: Decorations,
    pub glyphs: BTreeMap<char, Glyph>,
    pub fallback: Glyph,
    pub max_width: usize,
}

pub fn builtin_source(name: &str) -> Result<&'static str> {
    BUILTINS
        .iter()
        .find(|(id, _)| *id == name)
        .map(|(_, text)| *text)
        .ok_or_else(|| format!("unknown built-in font {name:?}; use --list-fonts"))
}
pub fn builtin(name: &str) -> Result<Font> {
    Font::parse(builtin_source(name)?).map_err(|e| format!("font {name}: {e}"))
}
pub fn load_file(path: &Path) -> Result<Font> {
    let text = read_utf8(path, MAX_FONT_BYTES)?;
    Font::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}
pub fn read_utf8(path: &Path, limit: usize) -> Result<String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|f| f.take(limit as u64 + 1).read_to_end(&mut bytes))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!("{} exceeds {limit} bytes", path.display()));
    }
    String::from_utf8(bytes).map_err(|e| format!("{} is not UTF-8: {e}", path.display()))
}

// Parse to Value first: unlike direct map deserialization, this rejects duplicate
// YAML keys. Tags have no meaning in this data-only format and are rejected.
fn reject_tags(value: &Value) -> Result<()> {
    match value {
        Value::Tagged(_) => return Err("YAML tags are not supported".into()),
        Value::Sequence(items) => {
            for item in items {
                reject_tags(item)?;
            }
        }
        Value::Mapping(map) => {
            for (k, v) in map {
                reject_tags(k)?;
                reject_tags(v)?;
            }
        }
        _ => (),
    }
    Ok(())
}
pub fn check_art(text: &str) -> Result<()> {
    if text.chars().any(|c| c.is_control()) {
        return Err("artwork must not contain control characters or tabs".into());
    }
    Ok(())
}
pub fn check_cell(text: &str) -> Result<()> {
    check_art(text)?;
    if UnicodeWidthStr::width(text) != 1 {
        return Err("decoration must occupy exactly one terminal column".into());
    }
    Ok(())
}
impl Font {
    pub fn parse(text: &str) -> Result<Self> {
        if text.len() > MAX_FONT_BYTES {
            return Err("font file exceeds 4 MiB".into());
        }
        let value: Value = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
        reject_tags(&value)?;
        let mut doc: Document = serde_yaml_ng::from_value(value).map_err(|e| e.to_string())?;
        if doc.format_version != 1 {
            return Err(format!("unsupported format_version {}", doc.format_version));
        }
        if doc.font.id.trim().is_empty() || doc.font.name.trim().is_empty() {
            return Err("font id and name must not be empty".into());
        }
        for text in [
            &doc.font.id,
            &doc.font.name,
            &doc.font.author,
            &doc.font.license,
            &doc.font.description,
        ] {
            check_art(text)?;
        }
        let m = &doc.metrics;
        if !(1..=128).contains(&m.height) || m.baseline >= m.height {
            return Err("height must be 1..128 and baseline must be a row within it".into());
        }
        if m.spacing > 128 || !(1..=512).contains(&m.space_width) {
            return Err("spacing must be 0..128 and space_width must be 1..512".into());
        }
        check_art(&m.separator)?;
        if m.separator.width() > 16 {
            return Err("separator must be at most 16 columns".into());
        }
        if let Some(b) = &mut doc.decorations.border {
            check_cell(&b.top)?;
            check_cell(&b.bottom)?;
            if !(1..=16).contains(&b.height) {
                return Err("border height must be 1..16".into());
            }
            if let Some(endcaps) = &mut b.endcaps {
                normalize_endcap(&mut endcaps.left).map_err(|e| format!("left endcap: {e}"))?;
                normalize_endcap(&mut endcaps.right).map_err(|e| format!("right endcap: {e}"))?;
            }
        }
        if let Some(u) = &doc.decorations.underline {
            check_cell(&u.char)?;
            if u.row >= m.height {
                return Err("underline row must be within font height".into());
            }
        }
        let mut glyphs = BTreeMap::new();
        let mut fallback = None;
        let mut max_width = m.space_width;
        for (key, value) in doc.glyphs {
            let ch =
                match &key {
                    Value::String(s) if s == "default" => None,
                    Value::Number(n) => Some(
                        n.as_u64()
                            .and_then(|n| u32::try_from(n).ok())
                            .and_then(char::from_u32)
                            .ok_or_else(|| format!("invalid Unicode code point: {n}"))?,
                    ),
                    _ => return Err(
                        "glyph keys must be Unicode integers or default (do not quote integers)"
                            .into(),
                    ),
                };
            if ch.is_some_and(char::is_control) {
                return Err(format!(
                    "control character U+{:04X} cannot be a glyph",
                    ch.unwrap() as u32
                ));
            }
            let label = ch.map_or_else(|| "default".into(), |c| format!("U+{:04X}", c as u32));
            let mut g: Glyph =
                serde_yaml_ng::from_value(value).map_err(|e| format!("{label}: {e}"))?;
            normalize_glyph(&mut g, &doc.metrics, &doc.decorations, ch == Some(' '))
                .map_err(|e| format!("{label}: {e}"))?;
            max_width = max_width.max(g.width.unwrap());
            if let Some(c) = ch {
                glyphs.insert(c, g);
            } else {
                fallback = Some(g);
            }
        }
        let fallback = fallback.ok_or("glyphs.default is required")?;
        // Space is semantic whitespace, never an unknown-character symbol.
        glyphs.entry(' ').or_insert_with(|| Glyph {
            width: Some(m.space_width),
            rows: vec![" ".repeat(m.space_width); m.height],
            underline: None,
            borders: None,
        });
        Ok(Self {
            meta: doc.font,
            metrics: doc.metrics,
            decorations: doc.decorations,
            glyphs,
            fallback,
            max_width,
        })
    }
    pub fn glyph(&self, ch: char) -> &Glyph {
        self.glyphs.get(&ch).unwrap_or(&self.fallback)
    }
    pub fn glyph_rows(&self, glyph: &Glyph, underline: bool) -> Vec<String> {
        let mut rows = glyph.rows.clone();
        if underline
            && let Some(u) = &self.decorations.underline
            && glyph.underline.as_ref().is_none_or(|g| g.enabled)
        {
            rows[u.row] = rows[u.row].replace(' ', &u.char);
            if let Some(g) = &glyph.underline {
                for (&row, text) in &g.rows {
                    rows[row] = text.clone();
                }
            }
        }
        rows
    }
}
fn normalize_endcap(cap: &mut Endcap) -> Result<()> {
    if !(1..=128).contains(&cap.top.len()) || !(1..=128).contains(&cap.bottom.len()) {
        return Err(
            "top and bottom must each contain 1..128 rows (use spaces for blank artwork)".into(),
        );
    }
    let width = cap.width.unwrap_or_else(|| {
        cap.top
            .iter()
            .chain(std::iter::once(&cap.middle))
            .chain(&cap.bottom)
            .map(|s| s.width())
            .max()
            .unwrap_or(0)
    });
    if !(1..=64).contains(&width) {
        return Err("width must be 1..64 terminal columns".into());
    }
    for row in cap
        .top
        .iter_mut()
        .chain(std::iter::once(&mut cap.middle))
        .chain(&mut cap.bottom)
    {
        check_art(row)?;
        if row.width() > width {
            return Err(format!("row exceeds endcap width {width}"));
        }
        row.push_str(&" ".repeat(width - row.width()));
    }
    cap.width = Some(width);
    Ok(())
}
fn normalize_glyph(
    g: &mut Glyph,
    m: &Metrics,
    decorations: &Decorations,
    space: bool,
) -> Result<()> {
    if g.rows.len() > m.height {
        return Err(format!("has {} rows, height is {}", g.rows.len(), m.height));
    }
    let inferred = g.rows.iter().map(|s| s.width()).max().unwrap_or(0);
    let width = g.width.unwrap_or(if space && inferred == 0 {
        m.space_width
    } else {
        inferred
    });
    if !(1..=512).contains(&width) {
        return Err("width must be 1..512; empty glyphs need an explicit width".into());
    }
    let pad = |s: &mut String| -> Result<()> {
        check_art(s)?;
        if s.width() > width {
            return Err(format!("row exceeds glyph width {width}"));
        }
        s.push_str(&" ".repeat(width - s.width()));
        Ok(())
    };
    for row in &mut g.rows {
        pad(row)?;
    }
    g.rows.resize(m.height, " ".repeat(width));
    if let Some(u) = &mut g.underline {
        if decorations.underline.is_none() {
            return Err("underline override requires font underline metadata".into());
        }
        if !u.enabled && !u.rows.is_empty() {
            return Err("disabled underline cannot define replacement rows".into());
        }
        for (&row, text) in &mut u.rows {
            if row >= m.height {
                return Err("underline replacement row is outside font height".into());
            }
            pad(text)?;
        }
    }
    if let Some(b) = &mut g.borders {
        for text in [&mut b.top, &mut b.bottom].into_iter().flatten() {
            pad(text)?;
        }
    }
    g.width = Some(width);
    Ok(())
}
