use bigtext::{
    cli::{self, Action},
    font::{self, Font},
    render::{self, Color, Options, Span, Style},
};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

const FONT: &str = include_str!("fixtures/endcaps.yaml");
fn style(yaml: &str) -> Style {
    Style::new(Arc::new(Font::parse(yaml).unwrap()))
}
fn options(top: bool, bottom: bool) -> Options {
    Options {
        top,
        bottom,
        endcaps: true,
        ..Options::default()
    }
}
fn draw(yaml: &str, text: &str, o: Options) -> Vec<String> {
    render::render(
        &[Span {
            text: text.into(),
            style: style(yaml),
        }],
        &o,
        false,
    )
    .unwrap()
}
fn left(row: &str, width: usize) -> String {
    row.chars().take(width).collect()
}
fn right(row: &str, width: usize) -> String {
    row.chars().skip(row.chars().count() - width).collect()
}
fn center(row: &str, left: usize, right: usize) -> String {
    row.chars()
        .skip(left)
        .take(row.chars().count() - left - right)
        .collect()
}

#[test]
fn full_frame_joins_the_fonts_corners_with_its_middle_artwork() {
    assert_eq!(
        draw(FONT, "X", options(true, true)),
        vec![
            " t---T  ", "t  a  T ", "l  b   r", "l  C   r", "l  d   r", "b  e  B ", " b===B  "
        ]
    );
}

#[test]
fn single_border_has_only_its_own_corners_and_keeps_blank_side_width() {
    let top = draw(FONT, "X", options(true, false));
    assert_eq!(
        top,
        vec![
            " t---T  ", "t  a  T ", "   b    ", "   C    ", "   d    ", "   e    "
        ]
    );
    let bottom = draw(FONT, "X", options(false, true));
    assert_eq!(
        bottom,
        vec![
            "   a    ", "   b    ", "   C    ", "   d    ", "b  e  B ", " b===B  "
        ]
    );
    assert!(top.iter().chain(&bottom).all(|row| row.width() == 8));
    let none = draw(FONT, "X", options(false, false));
    assert_eq!(none, draw(FONT, "X", Options::default()));
    assert_eq!(none[0], " a ");
    let disabled = Options {
        endcaps: false,
        ..options(true, true)
    };
    assert_eq!(
        draw(FONT, "X", disabled),
        vec!["---", " a ", " b ", " C ", " d ", " e ", "==="]
    );
}

#[test]
fn tall_corners_expand_the_frame_without_clipping_or_stretching_glyphs() {
    let short = FONT
        .replace("height: 5, baseline: 3", "height: 1, baseline: 0")
        .replace("['a', 'b', 'C', 'd', 'e']", "['X']")
        .replace("[' t', 't ']", "[' t', 't ', 'u ']")
        .replace("[' B ', 'B  ']", "[' V ', ' W ', ' B ', 'B  ']");
    let rows = draw(&short, "X", options(true, true));
    assert_eq!(rows.len(), 7); // three top rows plus four bottom rows
    assert_eq!(left(&rows[2], 2), "u ");
    assert_eq!(right(&rows[3], 3), " V ");
    assert_eq!(rows.iter().filter(|r| r.contains('X')).count(), 1);
    assert_eq!(center(&rows[3], 2, 3), " X ");
    assert!(rows.iter().all(|r| r.width() == 8));
}

#[test]
fn border_thickness_repeats_the_outer_joining_rows() {
    let rows = draw(
        FONT,
        "X",
        Options {
            border_height: Some(2),
            ..options(true, true)
        },
    );
    assert_eq!(rows.len(), 9);
    assert_eq!(rows[0], " t---T  ");
    assert_eq!(rows[1], rows[0]);
    assert_eq!(left(&rows[2], 2), "t ");
    assert_eq!(rows[8], " b===B  ");
    assert_eq!(rows[7], rows[8]);
    assert_eq!(left(&rows[6], 2), "b ");
    let top = draw(
        FONT,
        "X",
        Options {
            border_height: Some(2),
            ..options(true, false)
        },
    );
    assert!(
        top[3..]
            .iter()
            .all(|r| left(r, 2) == "  " && right(r, 3) == "   ")
    );
}

#[test]
fn mixed_fonts_use_first_and_last_characters_for_side_art_and_color() {
    let mut a = style(FONT);
    a.color = Some(Color::parse("red").unwrap());
    let other = FONT
        .replace("['T  ', ' T ']", "['U  ', ' U ']")
        .replace("middle: '  r'", "middle: '  R'");
    let mut b = style(&other);
    b.color = Some(Color::parse("blue").unwrap());
    let spans = [
        Span {
            text: "A".into(),
            style: a,
        },
        Span {
            text: "B".into(),
            style: b,
        },
    ];
    let rows = render::render(&spans, &options(true, true), false).unwrap();
    assert_eq!(left(&rows[0], 2), " t");
    assert_eq!(right(&rows[0], 3), "U  ");
    assert_eq!(right(&rows[3], 3), "  R");
    let colored = render::render(&spans, &options(true, true), true).unwrap();
    assert!(colored[0].starts_with("\x1b[91m t\x1b[0m"));
    assert!(colored[0].ends_with("\x1b[94mU  \x1b[0m"));
}

#[test]
fn absent_endcap_metadata_gets_a_straight_one_column_fallback() {
    let yaml = "format_version: 1\nfont: {id: fallback, name: Fallback}\nmetrics: {height: 2, baseline: 0}\ndecorations: {border: {top: '-', bottom: '='}}\nglyphs: {default: {rows: ['X', 'Y']}}\n";
    assert_eq!(
        draw(yaml, "?", options(true, true)),
        vec!["-----", "- X -", "- Y -", "====="]
    );
    assert_eq!(
        draw(yaml, "?", options(true, false)),
        vec!["-----", "  X  ", "  Y  "]
    );
    assert_eq!(
        draw(yaml, "?", options(false, true)),
        vec!["  X  ", "  Y  ", "====="]
    );
}

#[test]
fn overrides_preserve_border_spaces_and_apply_to_custom_glyph_art_too() {
    let overrides = Options {
        top_char: Some("#".into()),
        bottom_char: Some("+".into()),
        left_endcap_char: Some("<".into()),
        right_endcap_char: Some(">".into()),
        ..options(true, true)
    };
    let rows = draw(FONT, "X", overrides);
    assert_eq!(rows[0], " <###>  ");
    assert_eq!(rows[2], "<  b   >");
    assert_eq!(rows[6], " <+++>  ");
    let custom = FONT.replace(
        "rows: ['a', 'b', 'C', 'd', 'e']",
        "rows: [' X ']\n    borders: {top: 'x y', bottom: ' z '}",
    );
    let rows = draw(
        &custom,
        "?",
        Options {
            top_char: Some("#".into()),
            bottom_char: Some("+".into()),
            top: true,
            bottom: true,
            ..Options::default()
        },
    );
    assert_eq!(rows[0], "## ##");
    assert_eq!(rows[6], "+ + +");
    assert!(rows[1].contains('X'));
    let transparent = FONT.replace("top: '-'", "top: ' '");
    assert_eq!(
        center(
            &draw(
                &transparent,
                "X",
                Options {
                    top_char: Some("#".into()),
                    ..options(true, false)
                }
            )[0],
            2,
            3
        ),
        "   "
    );
}

#[test]
fn endcap_masks_keep_terminal_width_for_wide_and_combining_artwork() {
    let wide = FONT
        .replace("middle: 'l '", "middle: '界'")
        .replace("middle: '  r'", "middle: ' e\u{0301} '");
    let rows = draw(
        &wide,
        "X",
        Options {
            left_endcap_char: Some("=".into()),
            right_endcap_char: Some("!".into()),
            ..options(true, true)
        },
    );
    assert_eq!(rows[3], "== C  ! ");
    assert!(rows.iter().all(|r| r.width() == 8));
}

#[test]
fn endcap_configuration_is_strict_and_rows_are_padded_to_side_width() {
    for (from, to) in [
        ("top: [' t', 't ']", "top: []"),
        ("bottom: ['b ', ' b']", "bottom: []"),
        ("middle: 'l '", "middle: 'l '\n        width: 1"),
        ("middle: 'l '", "middle: 'l '\n        width: 65"),
        ("middle: 'l '", "middlle: 'l '"),
        ("middle: 'l '", "middle: \"\\t\""),
        ("middle: 'l '", "middle: 'l '\n        middle: 'x '"),
    ] {
        assert!(
            Font::parse(&FONT.replace(from, to)).is_err(),
            "{from} -> {to}"
        );
    }
    let oversized = format!("top: [{}]", vec!["'t'"; 129].join(", "));
    assert!(Font::parse(&FONT.replace("top: [' t', 't ']", &oversized)).is_err());
    let padded = FONT.replace("middle: 'l '", "middle: 'l'");
    assert_eq!(draw(&padded, "X", options(true, true))[3], "l  C   r");
}

#[test]
fn border_options_are_global_last_wins_and_do_not_enable_unrequested_borders() {
    let parse =
        |args: &[&str]| match cli::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap()
        {
            Action::Render(p) => p.options,
            _ => panic!(),
        };
    let o = parse(&[
        "--border-char",
        "=",
        "--endcap-char",
        "*",
        "--left-endcap-char",
        "+",
        "--top-border-char",
        "-",
        "X",
        "--border-endcaps",
    ]);
    assert!(!o.top && !o.bottom && o.endcaps);
    assert_eq!(o.top_char.as_deref(), Some("-"));
    assert_eq!(o.bottom_char.as_deref(), Some("="));
    assert_eq!(o.left_endcap_char.as_deref(), Some("+"));
    assert_eq!(o.right_endcap_char.as_deref(), Some("*"));
    let o = parse(&[
        "--left-endcap-char",
        "+",
        "--right-endcap-char",
        "-",
        "--endcap-char",
        "*",
        "--border-char",
        "=",
        "--border-endcaps",
        "--no-border-endcaps",
    ]);
    assert_eq!(o.left_endcap_char.as_deref(), Some("="));
    assert_eq!(o.right_endcap_char.as_deref(), Some("="));
    assert!(!o.endcaps);
    for args in [
        vec!["--endcap-char"],
        vec!["--left-endcap-char", "😀"],
        vec!["--right-endcap-char", "=="],
        vec!["--border-endcaps=yes"],
    ] {
        assert!(cli::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).is_err());
    }
}

#[test]
fn builtin_frames_preserve_center_artwork_and_compact_padding_in_all_modes() {
    for (name, _) in font::BUILTINS {
        for top in [false, true] {
            for bottom in [false, true] {
                for compact in [false, true] {
                    for underline in [false, true] {
                        let mut s = Style::new(Arc::new(font::builtin(name).unwrap()));
                        s.compact = compact;
                        s.underline = underline;
                        let spans = [Span {
                            text: "Agjpqy╭─╯".into(),
                            style: s,
                        }];
                        let framed = render::render(&spans, &options(top, bottom), false).unwrap();
                        let plain = render::render(
                            &spans,
                            &Options {
                                top,
                                bottom,
                                ..Options::default()
                            },
                            false,
                        )
                        .unwrap();
                        assert_eq!(framed.len(), plain.len());
                        for (framed, plain) in framed.iter().zip(&plain) {
                            assert_eq!(
                                if top || bottom {
                                    center(framed, 4, 4)
                                } else {
                                    framed.clone()
                                },
                                *plain
                            );
                            assert_eq!(
                                framed.width(),
                                plain.width() + if top || bottom { 8 } else { 0 }
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn wrappers_and_blank_logical_lines_stay_outside_the_frame() {
    let rows = draw(
        FONT,
        "X\n\nX",
        Options {
            prefix: "/* ".into(),
            suffix: " */".into(),
            ..options(true, true)
        },
    );
    assert_eq!(rows.len(), 23);
    assert!(
        rows.iter()
            .all(|r| r.starts_with("/* ") && r.ends_with(" */"))
    );
    assert_eq!(rows[0], "/*  t---T   */");
    assert_eq!(rows[7], "/*  */"); // interline gap has no frame
    assert_eq!(rows[8], "/*  tT   */"); // blank logical line retains its active font's frame
}
