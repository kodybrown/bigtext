use bigtext::{
    cli::{self, Action, Source},
    escape,
    font::{self, Font},
    render::{self, Options, Span, Style},
};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

fn style(name: &str) -> Style {
    Style::new(Arc::new(font::builtin(name).unwrap()))
}
fn render(text: &str, s: Style, o: Options) -> Vec<String> {
    render::render(
        &[Span {
            text: text.into(),
            style: s,
        }],
        &o,
        false,
    )
    .unwrap()
}
fn plan(args: &[&str]) -> cli::Plan {
    match cli::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap() {
        Action::Render(p) => *p,
        _ => panic!("expected rendering"),
    }
}
fn render_plan(args: &[&str]) -> Vec<String> {
    let p = plan(args);
    let spans: Vec<_> = p
        .inputs
        .into_iter()
        .map(|i| Span {
            text: match i.source {
                Source::Text(t) => t,
                _ => panic!(),
            },
            style: i.style,
        })
        .collect();
    render::render(&spans, &p.options, false).unwrap()
}
const MICRO: &str = include_str!("../examples/micro.yaml");

#[test]
fn every_imported_glyph_matches_csharp_normal_and_underlined_artwork() {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/csharp-glyphs.json")).unwrap();
    for (name, _) in font::BUILTINS {
        let font = font::builtin(name).unwrap();
        for (key, expected) in fixtures[*name].as_object().unwrap() {
            let glyph = if key == "default" {
                &font.fallback
            } else {
                font.glyph(char::from_u32(key.parse().unwrap()).unwrap())
            };
            for (underline, field) in [(false, "normal"), (true, "underlined")] {
                let rows: Vec<String> = serde_json::from_value(expected[field].clone()).unwrap();
                assert_eq!(
                    font.glyph_rows(glyph, underline),
                    rows,
                    "{name} {key} {field}"
                );
            }
        }
    }
}
#[test]
fn all_glyphs_and_style_combinations_have_rectangular_output() {
    for (name, _) in font::BUILTINS {
        for underline in [false, true] {
            for narrow in [false, true] {
                for top in [false, true] {
                    for bottom in [false, true] {
                        let mut s = style(name);
                        s.literal = true;
                        s.underline = underline;
                        if narrow {
                            s.spacing = Some(0);
                        }
                        let text: String = s.font.glyphs.keys().collect();
                        let rows = render(
                            &text,
                            s,
                            Options {
                                top,
                                bottom,
                                ..Options::default()
                            },
                        );
                        let width = rows[0].width();
                        assert!(width > 0);
                        assert!(
                            rows.iter().all(|r| r.width() == width),
                            "{name} u={underline} n={narrow} t={top} b={bottom}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn mixed_fonts_reserve_ascent_and_descent_but_keep_their_own_underlines() {
    let a=Font::parse("format_version: 1\nfont: {id: a, name: A}\nmetrics: {height: 2, baseline: 0, spacing: 0}\ndecorations: {underline: {row: 1, char: '~'}}\nglyphs: {default: {rows: ['A', ' ']}}\n").unwrap();
    let b=Font::parse("format_version: 1\nfont: {id: b, name: B}\nmetrics: {height: 3, baseline: 2, spacing: 0}\ndecorations: {underline: {row: 0, char: '_'}}\nglyphs: {default: {rows: [' ', 'b', 'B']}}\n").unwrap();
    let mut sa = Style::new(Arc::new(a));
    sa.underline = true;
    let mut sb = Style::new(Arc::new(b));
    sb.underline = true;
    let rows = render::render(
        &[
            Span {
                text: "x".into(),
                style: sa,
            },
            Span {
                text: "x".into(),
                style: sb,
            },
        ],
        &Options {
            top: true,
            bottom: true,
            ..Options::default()
        },
        false,
    )
    .unwrap();
    assert_eq!(rows, vec!["░░░░", "  __", "  b ", " AB ", "~~  ", "░░░░"]);
}
#[test]
fn ordered_options_persist_and_reset_does_not_reset_global_borders() {
    let p = plan(&[
        "--border",
        "--underline",
        "--color",
        "red",
        "--narrow",
        "--tall",
        "A",
        "--ultra",
        "B",
        "--no-underline",
        "C",
        "--literal",
        "\\u123",
        "--reset-style",
        "D",
    ]);
    assert!(p.options.top && p.options.bottom);
    assert_eq!(p.inputs[0].style.font.meta.id, "tall");
    assert_eq!(p.inputs[1].style.font.meta.id, "ultra");
    assert!(p.inputs[1].style.underline);
    assert!(p.inputs[1].style.color.is_some());
    assert_eq!(p.inputs[1].style.spacing, Some(0));
    assert!(!p.inputs[2].style.underline);
    assert!(p.inputs[3].style.literal);
    let reset = &p.inputs[4].style;
    assert_eq!(reset.font.meta.id, "basic");
    assert!(!reset.underline && !reset.literal && reset.color.is_none() && reset.spacing.is_none());
}
#[test]
fn text_is_exact_and_flags_can_follow_text() {
    assert_eq!(
        render_plan(&["hello ", "world"]),
        render_plan(&["hello world"])
    );
    assert_eq!(render_plan(&["A", "B"]), render_plan(&["AB"]));
    assert_ne!(
        render_plan(&["A", "--underline", "B"]),
        render_plan(&["AB"])
    );
    assert_eq!(
        render_plan(&["--literal", "--", "--hello"]),
        render("--hello", style("basic"), Options::default())
    );
}
#[test]
fn global_border_flags_are_last_wins_and_character_does_not_consume_text() {
    assert_eq!(
        render_plan(&["--border", "A"]),
        render_plan(&["A", "--border"])
    );
    assert_eq!(
        render_plan(&["--border", "A", "--no-border"]),
        render_plan(&["A"])
    );
    let rows = render_plan(&["--border", "--border-char", "=", "A"]);
    assert!(rows[0].chars().all(|c| c == '='));
    assert!(rows.last().unwrap().chars().all(|c| c == '='));
    assert_eq!(render_plan(&["--border-char", "=", "A"]).len(), 5);
    assert_eq!(render_plan(&["--top-border", "A"]).len(), 6);
}
#[test]
fn escapes_decode_unicode_newlines_tabs_and_literal_backslashes() {
    assert_eq!(
        escape::decode(r"\u0123\u{123}\U0001F600\\\n\r\t").unwrap(),
        "ģģ😀\\\n\r\t"
    );
    for bad in [
        r"\u123",
        r"\u{}",
        r"\u{1234567}",
        r"\u{D800}",
        r"\U00110000",
        r"\x",
        "\\",
    ] {
        assert!(escape::decode(bad).is_err(), "{bad}");
    }
    assert_eq!(
        render_plan(&["--tall", r"\u{2500}"]),
        render_plan(&["--tall", "─"])
    );
    assert_eq!(
        render_plan(&["--literal", r"\u123"]),
        render_plan(&[r"\\u123"])
    );
}
#[test]
fn newlines_blank_lines_crlf_and_wrappers_are_consistent() {
    let mut s = style("basic");
    s.literal = true;
    let o = Options {
        line_spacing: 0,
        prefix: "// ".into(),
        suffix: " END".into(),
        ..Options::default()
    };
    let rows = render("A\r\n\r\nB\n", s.clone(), o.clone());
    assert_eq!(rows.len(), 15);
    assert!(
        rows.iter()
            .all(|r| r.starts_with("// ") && r.ends_with(" END"))
    );
    assert_eq!(rows, render("A\n\nB", s, o));
    assert_eq!(render_plan(&["A\r", "\nB"]), render_plan(&["A\nB"]));
    assert!(render_plan(&[""]).is_empty());
    assert_eq!(render_plan(&["\n"]).len(), 5);
}
#[test]
fn tabs_use_input_columns_across_spans() {
    assert_eq!(
        render_plan(&["--tab-width", "4", "A", r"\tB"]),
        render_plan(&["A   B"])
    );
    assert_eq!(
        render_plan(&["--tab-width", "2", r"A\n\tB"]),
        render_plan(&["A\n  B"])
    );
}
#[test]
fn unknown_glyphs_use_default_and_legacy_codes_map_to_unicode() {
    for (name, _) in font::BUILTINS {
        let f = font::builtin(name).unwrap();
        assert_eq!(f.glyph('🦀').rows, f.fallback.rows);
        assert!(f.glyphs.contains_key(&'Ç'));
        assert!(f.glyphs.contains_key(&'─'));
        assert!(!f.glyphs.contains_key(&'\u{0080}'));
        assert!(f.glyphs.contains_key(&'☺'));
    }
}
#[test]
fn monospace_and_spacing_affect_geometry() {
    let a = render_plan(&["--basic", "iW"])[0].width();
    let b = render_plan(&["--basic", "--monospace", "iW"])[0].width();
    assert!(b > a);
    assert_eq!(render_plan(&["--narrow", "iW"])[0].width(), a - 1);
    assert_eq!(render_plan(&["--spacing", "2", "iW"])[0].width(), a + 1);
    let mut s = style("basic");
    s.monospace = true;
    s.monospace_width = Some(1);
    assert!(
        render::render(
            &[Span {
                text: "W".into(),
                style: s
            }],
            &Options::default(),
            false
        )
        .is_err()
    );
}
#[test]
fn external_font_supports_wide_art_and_non_bmp_integer_keys() {
    let f = Font::parse(MICRO).unwrap();
    assert_eq!(f.glyph('😀').width, Some(2));
    assert_eq!(f.glyph_rows(f.glyph('A'), true)[2], "|__|");
    assert_eq!(f.glyph(' ').width, Some(2));
}
#[test]
fn font_validation_rejects_bad_schema_and_dimensions() {
    for (from, to) in [
        ("format_version: 1", "format_version: 2"),
        ("baseline: 1", "baseline: 3"),
        ("height: 3", "height: 0"),
        ("row: 2", "row: 3"),
        ("  default:", "  63:"),
        ("  65:", "  '65':"),
        ("  65:", "  55296:"),
        ("  65:", "  1114112:"),
        ("  65:", "  9:"),
        ("  spacing: 1", "  spcaing: 1"),
        ("  id: micro", "  id: micro\n  id: duplicate"),
        ("    char: '_'", "    char: '😀'"),
        (
            "    rows: ['?', '?', ' ']",
            "    width: 1\n    rows: ['??', '?', ' ']",
        ),
        ("        2: '|__|'", "        3: '|__|'"),
        ("  name: Micro example", "  name: !custom Micro example"),
    ] {
        assert!(
            Font::parse(&MICRO.replace(from, to)).is_err(),
            "accepted {from} -> {to}"
        );
    }
}
#[test]
fn duplicate_glyph_and_override_keys_are_rejected() {
    let duplicate = format!("{MICRO}  65: {{rows: ['A']}}\n");
    assert!(Font::parse(&duplicate).is_err());
    assert!(
        Font::parse(&MICRO.replace("        2: '|__|'", "        2: '|__|'\n        2: 'oops'"))
            .is_err()
    );
}
#[test]
fn unsafe_controls_in_art_and_unknown_flags_are_errors() {
    let bad = MICRO.replace("['?', '?', ' ']", r#"["\e[31m", '?', ' ']"#);
    assert!(Font::parse(&bad).is_err());
    for args in [
        vec!["--missing"],
        vec!["--font"],
        vec!["--border=="],
        vec!["--stdin", "--stdin"],
        vec!["--spacing", "-1"],
        vec!["--border-char", "😀"],
    ] {
        assert!(
            cli::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).is_err(),
            "{args:?}"
        );
    }
}

#[test]
fn underline_suppression_and_border_overrides_preserve_custom_art() {
    let text = "format_version: 1\nfont: {id: custom, name: Custom}\nmetrics: {height: 2, baseline: 0, spacing: 1}\ndecorations: {border: {top: '-', bottom: '=', height: 2}, underline: {row: 1, char: '_'}}\nglyphs: {default: {rows: ['X', ' '], underline: {enabled: false, spacer: false}, borders: {top: ' ', bottom: 'v', spacer: false}}}\n";
    let mut s = Style::new(Arc::new(Font::parse(text).unwrap()));
    s.underline = true;
    let rows = render(
        "?",
        s,
        Options {
            top: true,
            bottom: true,
            ..Options::default()
        },
    );
    assert_eq!(rows, vec!["   ", "   ", " X ", "   ", " v ", " v "]);
}

#[test]
fn input_and_decoration_limits_fail_with_diagnostics() {
    let s = style("basic");
    assert!(
        render::render(
            &[Span {
                text: "\n".repeat(100_001),
                style: s.clone()
            }],
            &Options::default(),
            false
        )
        .is_err()
    );
    assert!(
        render::render(
            &[Span {
                text: "A".into(),
                style: s
            }],
            &Options {
                tab_width: 0,
                ..Options::default()
            },
            false
        )
        .is_err()
    );
}

fn spacer_style(id: &str, top: char, bottom: char, underline: char) -> Style {
    let yaml = format!(
        "format_version: 1\nfont: {{id: {id}, name: {id}}}\nmetrics: {{height: 2, baseline: 0, spacing: 1, space_width: 2}}\ndecorations: {{border: {{top: '{top}', bottom: '{bottom}'}}, underline: {{row: 1, char: '{underline}'}}}}\nglyphs:\n  default: {{rows: ['?', ' ']}}\n  65: {{rows: ['A', ' ']}}\n  66: {{rows: ['B', ' ']}}\n  67: {{rows: ['C', ' ']}}\n"
    );
    Style::new(Arc::new(Font::parse(&yaml).unwrap()))
}

#[test]
fn outer_padding_and_spacers_inherit_the_correct_character() {
    let mut a = spacer_style("a", '-', '=', '_');
    a.underline = true;
    let b = spacer_style("b", '*', '+', '~');
    let options = Options {
        top: true,
        bottom: true,
        ..Options::default()
    };
    let spans = [
        Span {
            text: "A".into(),
            style: a.clone(),
        },
        Span {
            text: "B".into(),
            style: b.clone(),
        },
    ];
    assert_eq!(
        render::render(&spans, &options, false).unwrap(),
        vec!["---**", " A B ", "___  ", "===++"]
    );
    a.underline = false;
    let mut b = b;
    b.underline = true;
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
    assert_eq!(render::render(&spans, &options, false).unwrap()[2], "   ~~");
    let override_options = Options {
        top_char: Some("#".into()),
        bottom_char: Some("!".into()),
        ..options
    };
    let rows = render::render(&spans, &override_options, false).unwrap();
    assert_eq!(rows[0], "#####");
    assert_eq!(rows[3], "!!!!!");
}

#[test]
fn compact_is_ordered_and_restores_configured_spacing_after_font_switches() {
    let p = plan(&[
        "--spacing",
        "3",
        "--compact",
        "A",
        "--tall",
        "B",
        "--no-compact",
        "C",
        "--reset-style",
        "D",
    ]);
    assert!(p.inputs[0].style.compact && p.inputs[1].style.compact);
    assert!(!p.inputs[2].style.compact && !p.inputs[3].style.compact);
    assert_eq!(p.inputs[2].style.spacing, Some(3));
    assert_eq!(p.inputs[3].style.spacing, None);
    let mut compact = spacer_style("a", '-', '=', '_');
    compact.compact = true;
    compact.spacing = Some(3);
    let mut normal = compact.clone();
    normal.compact = false;
    let rows = render::render(
        &[
            Span {
                text: "A".into(),
                style: compact,
            },
            Span {
                text: "BC".into(),
                style: normal,
            },
        ],
        &Options::default(),
        false,
    )
    .unwrap();
    assert_eq!(rows[0], " AB   C ");
}

#[test]
fn compact_and_zero_spacing_preserve_outer_padding_input_spaces_and_glyph_cells() {
    let mut s = spacer_style("a", '-', '=', '_');
    s.compact = true;
    s.underline = true;
    assert_eq!(
        render("AB", s.clone(), Options::default()),
        vec![" AB ", "____"]
    );
    assert_eq!(render("A B", s.clone(), Options::default())[0], " A  B ");
    assert_eq!(
        render("A", s.clone(), Options::default()),
        vec![" A ", "___"]
    );
    assert_eq!(
        render(
            "A\nB\n",
            s.clone(),
            Options {
                line_spacing: 0,
                ..Options::default()
            }
        ),
        vec![" A ", "___", " B ", "___"]
    );
    s.compact = false;
    s.spacing = Some(0);
    assert_eq!(
        render("AB", s.clone(), Options::default()),
        vec![" AB ", "____"]
    );
    s.compact = true;
    s.monospace = true;
    s.monospace_width = Some(3);
    assert_eq!(
        render("AB", s, Options::default()),
        vec![" A  B   ", "________"]
    );
    assert_eq!(
        render_plan(&["--compact", "--ultra", "Agjpqy"]),
        render_plan(&["--spacing", "0", "--ultra", "Agjpqy"])
    );
}

#[test]
fn spacer_color_comes_from_preceding_text_and_compact_removes_only_that_fragment() {
    let mut a = spacer_style("a", '-', '=', '_');
    a.color = Some(render::Color::parse("red").unwrap());
    let mut b = a.clone();
    b.color = Some(render::Color::parse("blue").unwrap());
    let spans = [
        Span {
            text: "A".into(),
            style: a.clone(),
        },
        Span {
            text: "B".into(),
            style: b.clone(),
        },
    ];
    let rows = render::render(&spans, &Options::default(), true).unwrap();
    let red = |s: &str| format!("\x1b[91m{s}\x1b[0m");
    let blue = |s: &str| format!("\x1b[94m{s}\x1b[0m");
    assert_eq!(
        rows[0],
        format!(
            "{}{}{}{}{}",
            red(" "),
            red("A"),
            red(" "),
            blue("B"),
            blue(" ")
        )
    );
    a.compact = true;
    let rows = render::render(
        &[
            Span {
                text: "A".into(),
                style: a,
            },
            Span {
                text: "B".into(),
                style: b,
            },
        ],
        &Options::default(),
        true,
    )
    .unwrap();
    assert_eq!(
        rows[0],
        format!("{}{}{}{}", red(" "), red("A"), blue("B"), blue(" "))
    );
}

#[test]
fn spacers_keep_the_preceding_fonts_underline_position_at_mixed_baselines() {
    let mut a = spacer_style("a", '-', '=', '_');
    a.underline = true;
    let b=Font::parse("format_version: 1\nfont: {id: b, name: B}\nmetrics: {height: 3, baseline: 2, spacing: 1}\ndecorations: {underline: {row: 0, char: '~'}}\nglyphs: {default: {rows: [' ', 'b', 'B']}}\n").unwrap();
    let mut b = Style::new(Arc::new(b));
    b.underline = true;
    let rows = render::render(
        &[
            Span {
                text: "A".into(),
                style: a,
            },
            Span {
                text: "B".into(),
                style: b,
            },
        ],
        &Options::default(),
        false,
    )
    .unwrap();
    assert_eq!(rows, vec!["   ~~", "   b ", " A B ", "___  "]);
}
