use crate::{
    Result,
    cli::ColorMode,
    escape,
    font::{Endcap, Font, Glyph},
};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(u8);
impl Color {
    pub fn parse(name: &str) -> Result<Self> {
        let normalized = name.to_ascii_lowercase().replace(['-', '_'], "");
        let code = match normalized.as_str() {
            "black" => 30,
            "darkred" => 31,
            "darkgreen" => 32,
            "darkyellow" => 33,
            "darkblue" => 34,
            "darkmagenta" => 35,
            "darkcyan" => 36,
            "gray" | "grey" => 37,
            "darkgray" | "darkgrey" => 90,
            "red" => 91,
            "green" => 92,
            "yellow" => 93,
            "blue" => 94,
            "magenta" => 95,
            "cyan" => 96,
            "white" => 97,
            _ => {
                return Err(format!(
                    "unknown color {name:?}; expected a console color such as red, dark-blue, gray, or white"
                ));
            }
        };
        Ok(Self(code))
    }
}
#[derive(Clone, Debug)]
pub struct Style {
    pub font: Arc<Font>,
    pub underline: bool,
    pub spacing: Option<usize>,
    pub compact: bool,
    pub monospace: bool,
    pub monospace_width: Option<usize>,
    pub color: Option<Color>,
    pub literal: bool,
}
impl Style {
    pub fn new(font: Arc<Font>) -> Self {
        Self {
            font,
            underline: false,
            spacing: None,
            compact: false,
            monospace: false,
            monospace_width: None,
            color: None,
            literal: false,
        }
    }
    fn spacing(&self) -> usize {
        self.spacing.unwrap_or(self.font.metrics.spacing)
    }
}
#[derive(Clone, Debug)]
pub struct Options {
    pub top: bool,
    pub bottom: bool,
    pub top_char: Option<String>,
    pub bottom_char: Option<String>,
    pub endcaps: bool,
    pub combine_borders: bool,
    pub left_endcap_char: Option<String>,
    pub right_endcap_char: Option<String>,
    pub border_height: Option<usize>,
    pub prefix: String,
    pub suffix: String,
    pub line_spacing: usize,
    pub tab_width: usize,
    pub color_mode: ColorMode,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            top: false,
            bottom: false,
            top_char: None,
            bottom_char: None,
            endcaps: false,
            combine_borders: false,
            left_endcap_char: None,
            right_endcap_char: None,
            border_height: None,
            prefix: String::new(),
            suffix: String::new(),
            line_spacing: 1,
            tab_width: 4,
            color_mode: ColorMode::Auto,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Span {
    pub text: String,
    pub style: Style,
}
#[derive(Clone)]
struct Unit {
    ch: char,
    style: Style,
}
struct LogicalLine {
    units: Vec<Unit>,
    empty_style: Style,
}
impl LogicalLine {
    fn last_style(&self) -> &Style {
        self.units.last().map_or(&self.empty_style, |u| &u.style)
    }
}

struct RenderedLine {
    rows: Vec<String>,
    // Terminal columns tracked during composition, excluding ANSI sequences.
    width: usize,
    top_height: usize,
    bottom_height: usize,
}

// The renderer returns data. It never reads a terminal, changes global color,
// opens a file, or writes output; callers can test or reuse it independently.
pub fn render(spans: &[Span], options: &Options, color_enabled: bool) -> Result<Vec<String>> {
    if !(1..=128).contains(&options.tab_width) || options.line_spacing > 128 {
        return Err("invalid tab width or line spacing".into());
    }
    if options
        .border_height
        .is_some_and(|h| !(1..=16).contains(&h))
    {
        return Err("invalid border height".into());
    }
    for c in [
        &options.top_char,
        &options.bottom_char,
        &options.left_endcap_char,
        &options.right_endcap_char,
    ]
    .into_iter()
    .flatten()
    {
        crate::font::check_cell(c)?;
    }
    crate::font::check_art(&options.prefix)?;
    crate::font::check_art(&options.suffix)?;
    let mut logical = Vec::new();
    let mut units = Vec::new();
    let mut column = 0;
    let mut after_cr = false;
    let mut count = 0;
    let mut last_style = spans.first().map(|s| s.style.clone());
    for span in spans {
        let text = if span.style.literal {
            span.text.clone()
        } else {
            escape::decode(&span.text)?
        };
        for ch in text.chars() {
            count += 1;
            if count > 100_000 {
                return Err("input exceeds 100,000 characters after tab expansion".into());
            }
            if after_cr && ch == '\n' {
                after_cr = false;
                continue;
            }
            after_cr = ch == '\r';
            last_style = Some(span.style.clone());
            if ch == '\n' || ch == '\r' {
                logical.push(LogicalLine {
                    units: std::mem::take(&mut units),
                    empty_style: span.style.clone(),
                });
                column = 0;
                continue;
            }
            if ch.is_control() && ch != '\t' {
                return Err(format!("unsupported control character U+{:04X}", ch as u32));
            }
            let (c, repeat) = if ch == '\t' {
                (' ', options.tab_width - column % options.tab_width)
            } else {
                (ch, 1)
            };
            count += repeat - 1;
            if count > 100_000 {
                return Err("input exceeds 100,000 characters after tab expansion".into());
            }
            for _ in 0..repeat {
                units.push(Unit {
                    ch: c,
                    style: span.style.clone(),
                });
            }
            column += repeat;
        }
    }
    // A final newline terminates a line. Consecutive newlines preserve blank
    // logical lines; an empty input does not create a phantom banner.
    if !units.is_empty() {
        logical.push(LogicalLine {
            units,
            empty_style: last_style.unwrap(),
        });
    }
    let mut output = Vec::new();
    let mut bytes = 0usize;
    if options.combine_borders && (options.top || options.bottom) && logical.len() > 1 {
        for row in render_block(&logical, options, color_enabled)? {
            push_row(&mut output, &mut bytes, row, options)?;
        }
        return Ok(output);
    }
    for (index, line) in logical.iter().enumerate() {
        if index != 0 {
            for _ in 0..options.line_spacing {
                push_row(&mut output, &mut bytes, String::new(), options)?;
            }
        }
        for row in render_line(line, options, color_enabled)?.rows {
            push_row(&mut output, &mut bytes, row, options)?;
        }
    }
    Ok(output)
}
fn push_row(output: &mut Vec<String>, bytes: &mut usize, row: String, o: &Options) -> Result<()> {
    if row.len() + o.prefix.len() + o.suffix.len() > 1024 * 1024 {
        return Err("a rendered row exceeds 1 MiB".into());
    }
    *bytes += row.len() + o.prefix.len() + o.suffix.len() + 1;
    if *bytes > 64 * 1024 * 1024 {
        return Err("rendered output exceeds 64 MiB".into());
    }
    output.push(format!("{}{}{}", o.prefix, row, o.suffix));
    Ok(())
}
fn styled(target: &mut String, text: &str, color: Option<Color>, enabled: bool) {
    if text.is_empty() {
        return;
    }
    if enabled && let Some(Color(code)) = color {
        target.push_str(&format!("\x1b[{code}m{text}\x1b[0m"));
        return;
    }
    target.push_str(text);
}
struct LineLayout<'a> {
    options: &'a Options,
    baseline: usize,
    top_height: usize,
    bottom_height: usize,
    height: usize,
}
impl LineLayout<'_> {
    fn offset(&self, style: &Style) -> usize {
        self.top_height + self.baseline - style.font.metrics.baseline
    }
    fn border_char(&self, unit: &Unit, top: bool) -> String {
        border_character(&unit.style, self.options, top)
    }
    fn spacer_cell(&self, owner: &Unit, row: usize) -> String {
        let glyph = owner.style.font.glyph(owner.ch);
        if row < self.top_height || row >= self.height - self.bottom_height {
            return if glyph.borders.as_ref().is_none_or(|b| b.spacer) {
                self.border_char(owner, row < self.top_height)
            } else {
                " ".into()
            };
        }
        if owner.style.underline
            && glyph.underline.as_ref().is_none_or(|u| u.spacer)
            && let Some(underline) = &owner.style.font.decorations.underline
            && row == self.offset(&owner.style) + underline.row
        {
            return underline.char.clone();
        }
        " ".into()
    }
}

fn append_piece(
    row: &mut String,
    text: &str,
    color: Option<Color>,
    colors: bool,
    bytes: &mut usize,
) -> Result<()> {
    let previous_len = row.len();
    styled(row, text, color, colors);
    *bytes += row.len() - previous_len;
    if *bytes > 64 * 1024 * 1024 {
        return Err("rendered output exceeds 64 MiB".into());
    }
    if row.len() > 1024 * 1024 {
        return Err("a rendered row exceeds 1 MiB".into());
    }
    Ok(())
}

// Padding and inter-glyph spacers are separate composition elements. The caller
// supplies the owning character: first for leading padding, last for trailing
// padding, and the preceding character for every internal spacer.
fn append_spacer(
    rows: &mut [String],
    owner: &Unit,
    width: usize,
    layout: &LineLayout<'_>,
    colors: bool,
    bytes: &mut usize,
) -> Result<()> {
    for (i, row) in rows.iter_mut().enumerate() {
        append_piece(
            row,
            &layout.spacer_cell(owner, i).repeat(width),
            owner.style.color,
            colors,
            bytes,
        )?;
    }
    Ok(())
}

fn render_line(line: &LogicalLine, o: &Options, colors: bool) -> Result<RenderedLine> {
    let styles: Vec<&Style> = if line.units.is_empty() {
        vec![&line.empty_style]
    } else {
        line.units.iter().map(|u| &u.style).collect()
    };
    let mut baseline = styles
        .iter()
        .map(|s| s.font.metrics.baseline)
        .max()
        .unwrap();
    let descent = styles
        .iter()
        .map(|s| s.font.metrics.height - s.font.metrics.baseline - 1)
        .max()
        .unwrap();
    let body_height = baseline + descent + 1;
    let border_height = o.border_height.unwrap_or_else(|| {
        styles
            .iter()
            .map(|s| s.font.decorations.border.as_ref().map_or(1, |b| b.height))
            .max()
            .unwrap()
    });
    let top_height = if o.top { border_height } else { 0 };
    let bottom_height = if o.bottom { border_height } else { 0 };
    let mut height = top_height + body_height + bottom_height;
    let sides = (o.endcaps && (o.top || o.bottom)).then(|| {
        (
            font_endcap(&styles[0].font, true),
            font_endcap(&styles.last().unwrap().font, false),
        )
    });
    if let Some((left, right)) = &sides {
        // Reserve both complete corner bands; neither side may be clipped or
        // overlap the opposing band's artwork. Extra space surrounds the text.
        let required = endcap_top_height(left, top_height)
            .max(endcap_top_height(right, top_height))
            + endcap_bottom_height(left, bottom_height)
                .max(endcap_bottom_height(right, bottom_height));
        let extra = required.saturating_sub(height);
        baseline += extra.div_ceil(2);
        height += extra;
    }
    let layout = LineLayout {
        options: o,
        baseline,
        top_height,
        bottom_height,
        height,
    };
    let mut rows = vec![String::new(); height];
    let mut rendered_width = if line.units.is_empty() { 0 } else { 2 };
    if let Some((left, right)) = &sides {
        rendered_width += left.width.unwrap() + right.width.unwrap();
    }
    let mut line_bytes = 0;
    if let Some((left, _)) = &sides {
        let cap_rows = endcap_rows(
            left,
            height,
            top_height,
            bottom_height,
            o.left_endcap_char.as_deref(),
        );
        for (row, cap) in rows.iter_mut().zip(cap_rows) {
            append_piece(row, &cap, styles[0].color, colors, &mut line_bytes)?;
        }
    }
    if let Some(first) = line.units.first() {
        append_spacer(&mut rows, first, 1, &layout, colors, &mut line_bytes)?;
    }
    for (index, unit) in line.units.iter().enumerate() {
        let style = &unit.style;
        let font = &style.font;
        let glyph = font.glyph(unit.ch);
        let actual_width = glyph.width.unwrap();
        let width = if style.monospace {
            style.monospace_width.unwrap_or(font.max_width)
        } else {
            actual_width
        };
        if width < actual_width {
            return Err(format!(
                "monospace width {width} is smaller than glyph U+{:04X} ({actual_width}) in {}",
                unit.ch as u32, font.meta.id
            ));
        }
        let spacing = style.spacing();
        if spacing > 128 || width > 512 {
            return Err("spacing or glyph cell width exceeds limits".into());
        }
        rendered_width += width + font.metrics.separator.width();
        let offset = layout.offset(style);
        let underline_row = font.decorations.underline.as_ref().map(|u| offset + u.row);
        let art = font.glyph_rows(glyph, style.underline);
        for (i, row) in rows.iter_mut().enumerate() {
            let body = if i < top_height {
                border_text(
                    glyph,
                    true,
                    &layout.border_char(unit, true),
                    width,
                    o.top_char.as_deref(),
                )
            } else if i >= height - bottom_height {
                border_text(
                    glyph,
                    false,
                    &layout.border_char(unit, false),
                    width,
                    o.bottom_char.as_deref(),
                )
            } else if i >= offset && i < offset + font.metrics.height {
                let text = &art[i - offset];
                let padding = if underline_row == Some(i)
                    && style.underline
                    && glyph.underline.as_ref().is_none_or(|u| u.enabled)
                {
                    font.decorations
                        .underline
                        .as_ref()
                        .map_or(" ", |u| u.char.as_str())
                } else {
                    " "
                };
                format!("{}{}", text, padding.repeat(width - text.width()))
            } else {
                " ".repeat(width)
            };
            append_piece(row, &body, style.color, colors, &mut line_bytes)?;
            append_piece(
                row,
                &font.metrics.separator,
                style.color,
                colors,
                &mut line_bytes,
            )?;
        }
        if index + 1 < line.units.len() && !style.compact {
            rendered_width += spacing;
            append_spacer(&mut rows, unit, spacing, &layout, colors, &mut line_bytes)?;
        }
    }
    if let Some(last) = line.units.last() {
        append_spacer(&mut rows, last, 1, &layout, colors, &mut line_bytes)?;
    }
    if let Some((_, right)) = &sides {
        let cap_rows = endcap_rows(
            right,
            height,
            top_height,
            bottom_height,
            o.right_endcap_char.as_deref(),
        );
        for (row, cap) in rows.iter_mut().zip(cap_rows) {
            append_piece(
                row,
                &cap,
                styles.last().unwrap().color,
                colors,
                &mut line_bytes,
            )?;
        }
    }
    Ok(RenderedLine {
        rows,
        width: rendered_width,
        top_height,
        bottom_height,
    })
}
// Compose interiors first, then put one frame around their shared rectangle.
fn render_block(lines: &[LogicalLine], o: &Options, colors: bool) -> Result<Vec<String>> {
    let left_style = lines
        .iter()
        .find_map(|l| l.units.first())
        .map_or(&lines[0].empty_style, |u| &u.style);
    let right_style = lines
        .iter()
        .rev()
        .find_map(|l| l.units.last())
        .map_or(&lines.last().unwrap().empty_style, |u| &u.style);
    let sides = o.endcaps.then(|| {
        (
            font_endcap(&left_style.font, true),
            font_endcap(&right_style.font, false),
        )
    });
    let mut rendered = Vec::with_capacity(lines.len());
    let mut width = 0;
    let mut height = o.line_spacing * (lines.len() - 1);
    let mut raw_bytes = 0;
    for (index, line) in lines.iter().enumerate() {
        let inner = Options {
            endcaps: false,
            top: o.top && index == 0,
            bottom: o.bottom && index + 1 == lines.len(),
            ..o.clone()
        };
        let part = render_line(line, &inner, colors)?;
        width = width.max(part.width);
        height += part.rows.len();
        raw_bytes += part.rows.iter().map(String::len).sum::<usize>();
        if raw_bytes > 64 * 1024 * 1024 {
            return Err("rendered output exceeds 64 MiB".into());
        }
        rendered.push(part);
    }
    let top_height = rendered[0].top_height;
    let bottom_height = rendered.last().unwrap().bottom_height;
    let mut extra = 0;
    let mut side_width = 0;
    if let Some((left, right)) = &sides {
        let required = endcap_top_height(left, top_height)
            .max(endcap_top_height(right, top_height))
            + endcap_bottom_height(left, bottom_height)
                .max(endcap_bottom_height(right, bottom_height));
        extra = required.saturating_sub(height);
        height += extra;
        side_width = left.width.unwrap() + right.width.unwrap();
    }
    // Check the rectangle before allocating padding for many short/blank lines.
    let row_minimum = width + side_width + o.prefix.len() + o.suffix.len();
    if row_minimum > 1024 * 1024 {
        return Err("a rendered row exceeds 1 MiB".into());
    }
    if height
        .checked_mul(row_minimum + 1)
        .is_none_or(|n| n > 64 * 1024 * 1024)
    {
        return Err("rendered output exceeds 64 MiB".into());
    }
    let mut rows = Vec::with_capacity(height);
    let mut bytes = raw_bytes;
    for (index, part) in rendered.into_iter().enumerate() {
        if index > 0 {
            for _ in 0..o.line_spacing {
                let mut gap = String::new();
                append_piece(&mut gap, &" ".repeat(width), None, colors, &mut bytes)?;
                rows.push(gap);
            }
        }
        let part_height = part.rows.len();
        for (row_index, mut row) in part.rows.into_iter().enumerate() {
            let border = if row_index < part.top_height {
                Some(true)
            } else if row_index >= part_height - part.bottom_height {
                Some(false)
            } else {
                None
            };
            let style = lines[index].last_style();
            let (padding, color) = match border {
                Some(top) => (border_character(style, o, top), style.color),
                None => (" ".into(), None),
            };
            append_piece(
                &mut row,
                &padding.repeat(width - part.width),
                color,
                colors,
                &mut bytes,
            )?;
            rows.push(row);
        }
    }
    // Oversized corners add space around the whole block, not around each line.
    for _ in 0..extra.div_ceil(2) {
        rows.insert(top_height, " ".repeat(width));
    }
    for _ in 0..extra / 2 {
        rows.insert(rows.len() - bottom_height, " ".repeat(width));
    }
    if let Some((left, right)) = sides {
        let left_rows = endcap_rows(
            &left,
            height,
            top_height,
            bottom_height,
            o.left_endcap_char.as_deref(),
        );
        let right_rows = endcap_rows(
            &right,
            height,
            top_height,
            bottom_height,
            o.right_endcap_char.as_deref(),
        );
        let mut frame_bytes = 0;
        for ((row, left), right) in rows.iter_mut().zip(left_rows).zip(right_rows) {
            let mut framed = String::new();
            append_piece(
                &mut framed,
                &left,
                left_style.color,
                colors,
                &mut frame_bytes,
            )?;
            append_piece(&mut framed, row, None, colors, &mut frame_bytes)?;
            append_piece(
                &mut framed,
                &right,
                right_style.color,
                colors,
                &mut frame_bytes,
            )?;
            *row = framed;
        }
    }
    Ok(rows)
}

fn border_character(style: &Style, options: &Options, top: bool) -> String {
    let explicit = if top {
        &options.top_char
    } else {
        &options.bottom_char
    };
    let default = style.font.decorations.border.as_ref().map_or("░", |b| {
        if top {
            b.top.as_str()
        } else {
            b.bottom.as_str()
        }
    });
    replace_border_ink(default, explicit.as_deref())
}
fn border_text(
    glyph: &Glyph,
    top: bool,
    default: &str,
    width: usize,
    replacement: Option<&str>,
) -> String {
    let custom = glyph.borders.as_ref().and_then(|b| {
        if top {
            b.top.as_ref()
        } else {
            b.bottom.as_ref()
        }
    });
    match custom {
        Some(text) => format!(
            "{}{}",
            replace_border_ink(text, replacement),
            " ".repeat(width - text.width())
        ),
        None => default.repeat(width),
    }
}

// Spaces describe the border shape. Replace only its occupied terminal cells,
// measuring non-space runs as strings so wide and combining artwork keeps its
// width even when the replacement occupies a single column.
fn replace_border_ink(text: &str, replacement: Option<&str>) -> String {
    match replacement {
        Some(character) => text
            .split(' ')
            .map(|run| character.repeat(run.width()))
            .collect::<Vec<_>>()
            .join(" "),
        None => text.into(),
    }
}

fn font_endcap(font: &Font, left: bool) -> Endcap {
    if let Some(caps) = font
        .decorations
        .border
        .as_ref()
        .and_then(|b| b.endcaps.as_ref())
    {
        return if left {
            caps.left.clone()
        } else {
            caps.right.clone()
        };
    }
    let border = font.decorations.border.as_ref();
    let top = border.map_or("░", |b| b.top.as_str()).to_owned();
    let bottom = border.map_or("░", |b| b.bottom.as_str()).to_owned();
    Endcap {
        width: Some(1),
        top: vec![top.clone()],
        middle: top,
        bottom: vec![bottom],
    }
}
fn endcap_top_height(cap: &Endcap, border_height: usize) -> usize {
    if border_height == 0 {
        0
    } else {
        cap.top.len() + border_height - 1
    }
}
fn endcap_bottom_height(cap: &Endcap, border_height: usize) -> usize {
    if border_height == 0 {
        0
    } else {
        cap.bottom.len() + border_height - 1
    }
}
fn endcap_rows(
    cap: &Endcap,
    height: usize,
    top_height: usize,
    bottom_height: usize,
    replacement: Option<&str>,
) -> Vec<String> {
    let top_len = endcap_top_height(cap, top_height);
    let bottom_len = endcap_bottom_height(cap, bottom_height);
    let blank = " ".repeat(cap.width.unwrap());
    (0..height)
        .map(|row| {
            let art = if row < top_len {
                // Repeat the outermost joining row for thicker horizontal borders.
                &cap.top[if row < top_height {
                    0
                } else {
                    row - top_height + 1
                }]
            } else if row >= height - bottom_len {
                let bottom_row = row - (height - bottom_len);
                &cap.bottom[bottom_row.min(cap.bottom.len() - 1)]
            } else if top_height > 0 && bottom_height > 0 {
                &cap.middle
            } else {
                &blank
            };
            replace_border_ink(art, replacement)
        })
        .collect()
}
