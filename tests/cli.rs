use std::{
    io::Write,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};
fn run(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bigtext"))
        .args(args)
        .env_remove("NO_COLOR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
fn success(args: &[&str], input: &str) -> String {
    let output = run(args, input);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
struct Temp(std::path::PathBuf);
impl Temp {
    fn new(contents: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "bigtext-test-{}-{}.font",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, contents).unwrap();
        Self(path)
    }
    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
#[test]
fn ordinary_text_is_printed_and_missing_input_does_not_panic() {
    assert!(!success(&["Hello"], "").is_empty());
    assert!(success(&[], "").is_empty());
    assert!(success(&["--help"], "").contains("Usage:"));
    assert_eq!(success(&["--version"], ""), "bigtext 0.1.0\n");
}
#[test]
fn stdin_and_files_use_current_formatting_and_input_order() {
    let expected = success(&["--tall", "--underline", "AB"], "");
    assert_eq!(success(&["--tall", "--underline"], "AB\n"), expected);
    assert_eq!(
        success(&["--tall", "--underline", "A", "--stdin"], "B"),
        expected
    );
    let text = Temp::new("B");
    assert_eq!(
        success(
            &["--tall", "--underline", "A", "--text-file", text.path()],
            ""
        ),
        expected
    );
    assert_eq!(success(&["A"], "unconsumed"), success(&["A"], ""));
}
#[test]
fn export_reload_and_validate_every_builtin() {
    for name in ["basic", "tall", "ultra"] {
        let yaml = success(&["--export-font", name], "");
        assert!(yaml.starts_with('#'));
        let file = Temp::new(&yaml);
        assert!(success(&["--check-font", file.path()], "").starts_with("Valid font:"));
        assert_eq!(
            success(
                &["--font-file", file.path(), "--underline", "Agjpqy╭─╯"],
                ""
            ),
            success(&["--font", name, "--underline", "Agjpqy╭─╯"], "")
        );
        assert!(success(&["--font-info", file.path()], "").contains("Baseline:"));
    }
    assert!(success(&["--list-fonts"], "").contains("ultra"));
}
#[test]
fn color_is_clean_when_redirected_and_can_be_forced() {
    let plain = success(&["A"], "");
    assert_eq!(success(&["--color", "red", "A"], ""), plain);
    assert_eq!(
        success(&["--color-mode", "never", "--color", "red", "A"], ""),
        plain
    );
    let colored = success(
        &[
            "--color-mode",
            "always",
            "--color",
            "red",
            "A",
            "--no-color",
            "B",
        ],
        "",
    );
    assert!(colored.contains("\x1b[91m"));
    assert!(colored.contains("\x1b[0m"));
}
#[test]
fn errors_do_not_leak_partial_banners_or_panic() {
    for args in [
        vec!["A", r"\u123"],
        vec!["--font", "missing", "A"],
        vec!["--font-file", "/nonexistent/bigtext-font", "A"],
        vec!["--no-such-flag"],
        vec!["--pause", "A"],
        vec!["--monospace-width", "1", "W"],
    ] {
        let out = run(&args, "");
        assert!(!out.status.success(), "{args:?}");
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8_lossy(&out.stderr).starts_with("bigtext:"));
        assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
    }
}
#[test]
fn comments_and_blank_lines_are_wrapped() {
    let output = success(&["--block-comment", "A\n\nB"], "");
    assert!(
        output
            .lines()
            .all(|line| line.starts_with("/* ") && line.ends_with(" */"))
    );
    assert!(
        success(&["--lua-comment", "A"], "")
            .lines()
            .all(|line| line.starts_with("-- "))
    );
}
#[test]
fn verbose_uses_stderr_and_demo_renders() {
    let output = run(&["--verbose", "A"], "");
    assert!(output.status.success());
    assert_eq!(output.stdout, success(&["A"], "").as_bytes());
    assert!(String::from_utf8_lossy(&output.stderr).contains("output rows"));
    for name in ["basic", "tall", "ultra"] {
        assert!(success(&["--font", name, "--demo"], "").len() > 1000);
    }
}

#[test]
fn compact_flags_work_with_ordered_cli_sources_and_restore_spacing() {
    let compact = success(&["--border", "--underline", "--compact", "AB"], "");
    assert_eq!(
        compact,
        success(&["--border", "--underline", "--narrow", "AB"], "")
    );
    assert_eq!(
        compact,
        success(&["--border", "--underline", "--compact"], "AB")
    );
    assert_eq!(
        success(
            &["--spacing", "3", "--compact", "A", "--no-compact", "BC"],
            ""
        ),
        success(&["--spacing", "0", "A", "--spacing", "3", "BC"], "")
    );
    assert!(success(&["--help"], "").contains("--compact / --no-compact"));
}

#[test]
fn endcaps_cli_supports_partial_frames_overrides_and_external_font_roundtrips() {
    let args = [
        "--ultra",
        "--border",
        "--border-endcaps",
        "--border-char",
        "=",
        "--block-comment",
        "bigtext",
    ];
    let output = success(&args, "");
    assert!(
        output
            .lines()
            .all(|r| r.starts_with("/* ") && r.ends_with(" */"))
    );
    assert!(output.starts_with("/*   ==="));
    assert!(!output.contains('░'));
    let plain = success(&["--ultra", "--border", "bigtext"], "");
    assert_eq!(
        plain,
        success(
            &[
                "--ultra",
                "--border",
                "--border-endcaps",
                "bigtext",
                "--no-border-endcaps"
            ],
            ""
        )
    );
    for name in ["basic", "tall", "ultra"] {
        let file = Temp::new(&success(&["--export-font", name], ""));
        assert!(success(&["--font-info", file.path()], "").contains("Endcaps: font-defined"));
        for border in ["--border", "--top-border", "--bottom-border"] {
            assert_eq!(
                success(
                    &[
                        "--font",
                        name,
                        border,
                        "--border-endcaps",
                        "--compact",
                        "--underline",
                        "Agjpqy"
                    ],
                    ""
                ),
                success(
                    &[
                        "--font-file",
                        file.path(),
                        border,
                        "--border-endcaps",
                        "--compact",
                        "--underline",
                        "Agjpqy"
                    ],
                    ""
                )
            );
        }
    }
    let ornate = Temp::new(include_str!("../examples/ornate-frame.yaml"));
    let top = success(
        &[
            "--font-file",
            ornate.path(),
            "--top-border",
            "--border-endcaps",
            "bigtext",
        ],
        "",
    );
    assert!(top.starts_with("  █▀▀"));
    assert!(
        top.lines()
            .skip(3)
            .all(|r| r.starts_with("    ") && r.ends_with("    "))
    );
    let o = run(&["--left-endcap-char", "xx", "A"], "");
    assert!(!o.status.success() && o.stdout.is_empty());
}
