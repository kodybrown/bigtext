use bigtext::{
    font::{self, Font},
    render::{self, Options, Span, Style},
};
use std::{path::PathBuf, sync::Arc};
use unicode_width::UnicodeWidthStr;

const REQUIRED: &str = "abcABCxyzXYZZ0123456789bigtextBIGTEXT!?.,";
const EXTRAS: &str = " -_:/jpqmMwW";
const SAMPLES: &[&str] = &[
    "abcABCxyzXYZZ",
    "0123456789",
    "bigtext BIGTEXT",
    "!?., -_:/",
    "jpq mMwW",
];

fn examples() -> Vec<(PathBuf, Font)> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut files: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("yaml" | "yml")
            )
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no example fonts found");
    files
        .into_iter()
        .map(|path| {
            let font = font::load_file(&path).unwrap();
            (path, font)
        })
        .collect()
}

#[test]
fn every_example_has_complete_metadata_and_real_glyphs_for_the_sample_repertoire() {
    for (path, font) in examples() {
        assert!(
            !font.meta.author.is_empty()
                && !font.meta.license.is_empty()
                && !font.meta.description.is_empty(),
            "{}: incomplete metadata",
            path.display()
        );
        assert!(
            font.decorations.underline.is_some(),
            "{}: missing underline metadata",
            path.display()
        );
        assert!(
            font.decorations
                .border
                .as_ref()
                .and_then(|b| b.endcaps.as_ref())
                .is_some(),
            "{}: missing endcap artwork",
            path.display()
        );
        for ch in REQUIRED.chars().chain(EXTRAS.chars()) {
            // Looking up font.glyph(ch) alone could silently return the fallback.
            assert!(
                font.glyphs.contains_key(&ch),
                "{}: missing {:?} (U+{:04X})",
                path.display(),
                ch,
                ch as u32
            );
            if ch != ' ' {
                assert!(
                    font.glyph(ch).rows.iter().any(|row| !row.trim().is_empty()),
                    "{}: empty artwork for {ch:?}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn examples_render_the_samples_with_underlines_compact_spacing_and_partial_frames() {
    for (path, font) in examples() {
        let font = Arc::new(font);
        for sample in SAMPLES {
            for (top, bottom) in [(false, false), (true, false), (false, true), (true, true)] {
                for compact in [false, true] {
                    let mut s = Style::new(font.clone());
                    s.compact = compact;
                    let options = Options {
                        top,
                        bottom,
                        endcaps: true,
                        ..Options::default()
                    };
                    let normal = render::render(
                        &[Span {
                            text: (*sample).into(),
                            style: s.clone(),
                        }],
                        &options,
                        false,
                    )
                    .unwrap();
                    s.underline = true;
                    let underlined = render::render(
                        &[Span {
                            text: (*sample).into(),
                            style: s,
                        }],
                        &options,
                        false,
                    )
                    .unwrap();
                    assert_eq!(
                        normal.len(),
                        underlined.len(),
                        "{}: underline changed height",
                        path.display()
                    );
                    let width = normal[0].width();
                    assert!(
                        normal.iter().chain(&underlined).all(|r| r.width() == width),
                        "{}: ragged output for {sample}",
                        path.display()
                    );
                }
            }
        }
    }
}

#[test]
fn ornate_examples_sample_glyphs_retain_ultras_original_artwork_and_underline_variants() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/ornate-frame.yaml");
    let example = font::load_file(&path).unwrap();
    let ultra = font::builtin("ultra").unwrap();
    for ch in REQUIRED.chars().chain(EXTRAS.chars()) {
        for underlined in [false, true] {
            assert_eq!(
                example.glyph_rows(example.glyph(ch), underlined),
                ultra.glyph_rows(ultra.glyph(ch), underlined),
                "{ch:?}: changed Ultra artwork"
            );
        }
    }
}
