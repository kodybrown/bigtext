use bigtext::{
    Result,
    cli::{self, Action, ColorMode, Input, Source},
    font,
    render::{self, Span},
};
use std::{
    io::{self, IsTerminal, Read, Write},
    path::Path,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("bigtext: {error}");
        std::process::exit(2);
    }
}
fn run() -> Result<()> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "arguments must be valid UTF-8".to_owned())
        })
        .collect::<Result<_>>()?;
    let action = cli::parse(&args)?;
    let stdout = io::stdout();
    let terminal = stdout.is_terminal();
    let mut output = io::BufWriter::new(stdout.lock());
    let mut pause = false;
    let text = match action {
        Action::Help => cli::HELP.into(),
        Action::Version => format!("bigtext {}\n", env!("CARGO_PKG_VERSION")),
        Action::Export(name) => font::builtin_source(&name)?.into(),
        Action::List => {
            let mut out = String::new();
            for (name, _) in font::BUILTINS {
                let f = font::builtin(name)?;
                out.push_str(&format!(
                    "{name:<8} {} rows, baseline {}, {} glyphs — {}\n",
                    f.metrics.height,
                    f.metrics.baseline,
                    f.glyphs.len(),
                    f.meta.name
                ));
            }
            out
        }
        Action::Info(name) => {
            let f = if font::BUILTINS.iter().any(|(id, _)| *id == name) {
                font::builtin(&name)?
            } else {
                font::load_file(Path::new(&name))?
            };
            format!(
                "{} ({})\nAuthor: {}\nLicense: {}\nHeight: {}\nBaseline: {}\nDefault spacing: {}\nSpace width: {}\nMaximum glyph width: {}\nGlyphs: {} + default\nUnderline row: {}\nEndcaps: {}\n{}\n",
                f.meta.name,
                f.meta.id,
                f.meta.author,
                f.meta.license,
                f.metrics.height,
                f.metrics.baseline,
                f.metrics.spacing,
                f.metrics.space_width,
                f.max_width,
                f.glyphs.len(),
                f.decorations
                    .underline
                    .as_ref()
                    .map_or("none".into(), |u| u.row.to_string()),
                if f.decorations
                    .border
                    .as_ref()
                    .and_then(|b| b.endcaps.as_ref())
                    .is_some()
                {
                    "font-defined"
                } else {
                    "straight fallback"
                },
                f.meta.description
            )
        }
        Action::Check(path) => {
            let f = font::load_file(Path::new(&path))?;
            format!(
                "Valid font: {} ({}, {} glyphs + default)\n",
                f.meta.name,
                path,
                f.glyphs.len()
            )
        }
        Action::Render(mut plan) => {
            if plan.pause && (!io::stdin().is_terminal() || !io::stderr().is_terminal()) {
                return Err("--pause requires an interactive stdin and stderr".into());
            }
            pause = plan.pause;
            if plan.inputs.is_empty() {
                if io::stdin().is_terminal() {
                    return write_output(&mut output, cli::HELP);
                }
                plan.inputs.push(Input {
                    source: Source::Stdin,
                    style: plan.final_style,
                });
            }
            let mut spans = Vec::new();
            for input in plan.inputs {
                let text = match input.source {
                    Source::Text(s) => s,
                    Source::File(path) => font::read_utf8(Path::new(&path), 16 * 1024 * 1024)?,
                    Source::Stdin => {
                        let mut bytes = Vec::new();
                        io::stdin()
                            .take(16 * 1024 * 1024 + 1)
                            .read_to_end(&mut bytes)
                            .map_err(|e| format!("stdin: {e}"))?;
                        if bytes.len() > 16 * 1024 * 1024 {
                            return Err("stdin exceeds 16 MiB".into());
                        }
                        String::from_utf8(bytes).map_err(|e| format!("stdin is not UTF-8: {e}"))?
                    }
                };
                spans.push(Span {
                    text,
                    style: input.style,
                });
            }
            let colors = match plan.options.color_mode {
                ColorMode::Always => true,
                ColorMode::Never => false,
                ColorMode::Auto => {
                    terminal
                        && std::env::var_os("NO_COLOR").is_none()
                        && std::env::var("TERM").as_deref() != Ok("dumb")
                }
            };
            let rows = render::render(&spans, &plan.options, colors)?;
            if plan.verbose {
                eprintln!(
                    "bigtext: {} text spans, {} output rows, color {}",
                    spans.len(),
                    rows.len(),
                    if colors { "enabled" } else { "disabled" }
                );
            }
            if rows.is_empty() {
                String::new()
            } else {
                rows.join("\n") + "\n"
            }
        }
    };
    write_output(&mut output, &text)?;
    if pause {
        let term = console::Term::stderr();
        term.write_str("Press any key to exit: ")
            .and_then(|_| term.read_key())
            .and_then(|_| term.write_line(""))
            .map_err(|e| format!("pause: {e}"))?;
    }
    Ok(())
}
fn write_output(output: &mut impl Write, text: &str) -> Result<()> {
    match output
        .write_all(text.as_bytes())
        .and_then(|_| output.flush())
    {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(format!("stdout: {e}")),
    }
}
