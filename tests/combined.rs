use bigtext::{
    font::Font,
    render::{self, Color, Options, Span, Style},
};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

const FONT: &str = include_str!("fixtures/endcaps.yaml");
fn style(yaml: &str) -> Style {
    Style::new(Arc::new(Font::parse(yaml).unwrap()))
}
fn options() -> Options {
    Options {
        top: true,
        bottom: true,
        endcaps: true,
        combine_borders: true,
        line_spacing: 0,
        ..Options::default()
    }
}
fn draw(text: &str, o: &Options) -> Vec<String> {
    render::render(
        &[Span {
            text: text.into(),
            style: style(FONT),
        }],
        o,
        false,
    )
    .unwrap()
}

#[test]
fn one_rectangle_preserves_bodies_and_pads_shorter_lines() {
    assert_eq!(
        draw("XX\nX", &options()),
        vec![
            " t-----T  ",
            "t  a a  T ",
            "l  b b   r",
            "l  C C   r",
            "l  d d   r",
            "l  e e   r",
            "l  a     r",
            "l  b     r",
            "l  C     r",
            "l  d     r",
            "b  e    B ",
            " b=====B  ",
        ]
    );
    assert_eq!(draw("X\nXX", &options())[0], " t-----T  ");
}

#[test]
fn gaps_and_blank_lines_are_inside_frame_and_wrappers() {
    let o = Options {
        line_spacing: 2,
        prefix: "/*".into(),
        suffix: "*/".into(),
        ..options()
    };
    let rows = draw("XX\n\nX\n", &o);
    assert_eq!(rows.len(), 21); // three five-row bodies, four gap rows, two borders
    assert!(rows.iter().all(|r| r.width() == 14));
    assert_eq!(rows[6], "/*l        r*/");
    assert_eq!(rows[8], rows[6]);
    assert_eq!(rows, draw("XX\n\nX", &o));
}

#[test]
fn partial_borders_only_draw_outer_corners() {
    let top = draw(
        "X\nX",
        &Options {
            bottom: false,
            ..options()
        },
    );
    assert_eq!(top.len(), 11);
    assert_eq!(top[0], " t---T  ");
    assert_eq!(top[1], "t  a  T ");
    assert_eq!(top[6], "   a    ");
    assert_eq!(top[10], "   e    ");
    let bottom = draw(
        "X\nX",
        &Options {
            top: false,
            ..options()
        },
    );
    assert_eq!(bottom[0], "   a    ");
    assert_eq!(bottom[9], "b  e  B ");
    assert_eq!(bottom[10], " b===B  ");
}

#[test]
fn unchanged_for_single_line_empty_input_or_no_borders() {
    for text in ["", "X", "XX\n"] {
        assert_eq!(
            draw(text, &options()),
            draw(
                text,
                &Options {
                    combine_borders: false,
                    ..options()
                }
            )
        );
    }
    let o = Options {
        top: false,
        bottom: false,
        ..options()
    };
    assert_eq!(
        draw("X\nXX", &o),
        draw(
            "X\nXX",
            &Options {
                combine_borders: false,
                ..o
            }
        )
    );
}

#[test]
fn wide_artwork_and_colors_keep_terminal_width_and_boundary_styles() {
    let second = FONT
        .replace("top: '-'", "top: '~'")
        .replace("bottom: '='", "bottom: '_'")
        .replace("'  r'", "'  R'")
        .replace(
            "['a', 'b', 'C', 'd', 'e']",
            "['界', '界', '界', '界', '界']",
        );
    let mut first_style = style(FONT);
    first_style.color = Some(Color::parse("red").unwrap());
    let spans = [
        Span {
            text: "XX\n".into(),
            style: first_style,
        },
        Span {
            text: "X".into(),
            style: style(&second),
        },
    ];
    let plain = render::render(&spans, &options(), false).unwrap();
    let colored = render::render(&spans, &options(), true).unwrap();
    assert!(plain.iter().all(|r| r.width() == 10));
    assert_eq!(plain[0], " t-----T  ");
    assert_eq!(plain.last().unwrap(), " b_____B  ");
    assert!(plain[6].ends_with('R'));
    assert!(colored[6].starts_with("\x1b[91ml \x1b[0m"));
    for (a, b) in plain.iter().zip(colored) {
        assert_eq!(*a, b.replace("\x1b[91m", "").replace("\x1b[0m", ""));
    }
}

#[test]
fn thick_borders_and_masks_apply_once_to_the_block() {
    let o = Options {
        border_height: Some(2),
        top_char: Some("+".into()),
        bottom_char: Some("-".into()),
        left_endcap_char: Some("[".into()),
        right_endcap_char: Some("]".into()),
        ..options()
    };
    let rows = draw("X\nX", &o);
    assert_eq!(rows.len(), 14);
    assert_eq!(rows[0], " [+++]  ");
    assert_eq!(rows[1], rows[0]);
    assert_eq!(rows[12], " [---]  ");
    assert_eq!(rows[13], rows[12]);
}

#[test]
fn oversized_corners_expand_only_the_whole_block() {
    let short = FONT
        .replace("height: 5, baseline: 3", "height: 1, baseline: 0")
        .replace("['a', 'b', 'C', 'd', 'e']", "['X']")
        .replace("[' t', 't ']", "[' t', 't ', 'u ', 'v ']");
    let rows = render::render(
        &[Span {
            text: "X\nX".into(),
            style: style(&short),
        }],
        &options(),
        false,
    )
    .unwrap();
    assert_eq!(rows.len(), 6);
    assert_eq!(rows.iter().filter(|r| r.contains('X')).count(), 2);
}

#[test]
fn large_padding_rectangle_is_rejected() {
    let text = format!("{}{}", "X".repeat(1000), "\n".repeat(8000));
    assert!(
        render::render(
            &[Span {
                text,
                style: style(FONT)
            }],
            &options(),
            false
        )
        .unwrap_err()
        .contains("64 MiB")
    );
}
