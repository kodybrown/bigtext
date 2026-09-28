use crate::{
    Result,
    font::{self, Font},
    render::{Color, Options, Style},
};
use std::{collections::HashMap, path::Path, sync::Arc};

pub const HELP: &str = r#"bigtext — compose large Unicode text

Usage: bigtext [OPTIONS | TEXT]...
Text arguments concatenate exactly. Styling affects subsequent text until changed.
With no text sources, read piped stdin; with an interactive stdin, show this help.

Fonts and span formatting:
  --basic | --tall | --ultra       Select an embedded font
  -f, --font NAME                 Select an embedded font by name
  --font-file PATH                Load a UTF-8 YAML font (any file extension)
  -u, --underline / --no-underline
  -n, --narrow                    Zero internal spacing; keep outer padding
  -w, --wide                      Restore the selected font's default spacing
  --spacing N                    Explicit internal spacing (0..128)
  --compact / --no-compact       Suppress/restore spacers after subsequent text
  -m, --monospace / --no-monospace Use the font's widest glyph as a fixed cell
  --monospace-width N             Explicit fixed cell width, never clips artwork
  -c, --color NAME / --no-color   16 terminal colors (e.g. red, dark-blue)
  --literal / --no-literal        Disable/enable backslash escape decoding
  --reset-style                  Restore Basic and all default span formatting

Global output settings (last setting wins for the complete output):
  -b, --border / --no-border      Both borders using font defaults
  --top-border / --no-top-border  Also accepts --border-top
  --bottom-border / --no-bottom-border  Also accepts --border-bottom
  --border-char CHAR             Replace ink on all borders/endcaps (one column)
  --top-border-char CHAR         Override just the top border character
  --bottom-border-char CHAR      Override just the bottom border character
  --border-endcaps / --no-border-endcaps  Enable/disable font-defined side caps
  --endcap-char CHAR             Replace ink on both endcaps
  --left-endcap-char CHAR        Replace ink on the left endcap
  --right-endcap-char CHAR       Replace ink on the right endcap
  --border-height N              Override border thickness (1..16)
  --prefix TEXT / --suffix TEXT  Literal wrappers on every output row
  --slash-comment | --hash-comment | --tick-comment | --lua-comment
  --block-comment | --html-comment | --no-comment
  --line-spacing N               Blank rows between logical lines (default 1)
  --tab-width N                  Input tab stops (default 4; 1..128)
  --color-mode auto|always|never  Default auto: color only on terminals

Input and utilities:
  --text-file PATH                Insert a UTF-8 text file with current formatting
  --stdin                        Insert stdin here, once
  --newline                      Start another logical line
  --                             Treat remaining arguments as text
  --list-fonts                    List embedded fonts
  --font-info NAME_OR_PATH        Inspect an embedded or external font
  --export-font NAME              Write embedded YAML to stdout
  --check-font PATH               Validate an external font
  --demo                         Insert a preview of the current font
  -v, --verbose                   Write rendering details to stderr
  -p, --pause                     Wait for a key on an interactive terminal
  -h, --help                      Show this help
  --version                      Show version

Escapes: \n \r \t \\ \u0123 \u{123} \U0001F600
Examples (Bash):
  bigtext --border-char '=' --border --basic 'hello ' --underline --tall W --no-underline 'orld!'
  bigtext --tall '\u{2500}'
  bigtext --export-font tall > my-font.yaml
  printf 'Hello\nWorld\n' | bigtext --font-file my-font.yaml
"#;

#[derive(Clone, Debug)]
pub enum Source {
    Text(String),
    File(String),
    Stdin,
}
#[derive(Clone, Debug)]
pub struct Input {
    pub source: Source,
    pub style: Style,
}
#[derive(Debug)]
pub struct Plan {
    pub inputs: Vec<Input>,
    pub final_style: Style,
    pub options: Options,
    pub verbose: bool,
    pub pause: bool,
}
#[derive(Debug)]
pub enum Action {
    Help,
    Version,
    List,
    Info(String),
    Export(String),
    Check(String),
    Render(Box<Plan>),
}

pub fn parse(args: &[String]) -> Result<Action> {
    let basic = Arc::new(font::builtin("basic")?);
    let default = Style::new(basic.clone());
    let mut style = default.clone();
    let mut cache: HashMap<String, Arc<Font>> = HashMap::from([("basic".into(), basic)]);
    let mut options = Options::default();
    let mut inputs = Vec::new();
    let mut action = None;
    let (mut verbose, mut pause, mut literal_args, mut stdin_used) = (false, false, false, false);
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if !literal_args && arg == "--" {
            literal_args = true;
            continue;
        }
        if literal_args || !arg.starts_with('-') || arg == "-" {
            inputs.push(Input {
                source: Source::Text(arg.clone()),
                style: style.clone(),
            });
            continue;
        }
        let (flag, attached) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(k, v)| (k, Some(v)));
        let takes_value = matches!(
            flag,
            "--font"
                | "-f"
                | "--font-file"
                | "--spacing"
                | "--monospace-width"
                | "--color"
                | "-c"
                | "--border-char"
                | "--top-border-char"
                | "--bottom-border-char"
                | "--border-top-char"
                | "--border-bottom-char"
                | "--endcap-char"
                | "--left-endcap-char"
                | "--right-endcap-char"
                | "--border-height"
                | "--prefix"
                | "--line-prefix"
                | "--pre-line"
                | "--suffix"
                | "--line-suffix"
                | "--post-line"
                | "--line-spacing"
                | "--tab-width"
                | "--color-mode"
                | "--text-file"
                | "--font-info"
                | "--export-font"
                | "--check-font"
        );
        if attached.is_some() && !takes_value {
            return Err(format!(
                "{flag} does not take a value; use --border-char for a border character"
            ));
        }
        let mut value = || -> Result<String> {
            if let Some(v) = attached {
                return Ok(v.into());
            }
            let v = args
                .get(i)
                .ok_or_else(|| format!("{flag} requires a value"))?
                .clone();
            i += 1;
            Ok(v)
        };
        match flag {
            "--basic" | "--tall" | "--ultra" | "--font" | "-f" | "--font-file" => {
                let file = flag == "--font-file";
                let name = if matches!(flag, "--font" | "-f" | "--font-file") {
                    value()?
                } else {
                    flag[2..].into()
                };
                let key = if file {
                    format!("file:{name}")
                } else {
                    name.clone()
                };
                if !cache.contains_key(&key) {
                    let font = if file {
                        font::load_file(Path::new(&name))?
                    } else {
                        font::builtin(&name)?
                    };
                    cache.insert(key.clone(), Arc::new(font));
                }
                style.font = cache[&key].clone();
            }
            "--underline" | "-u" => style.underline = true,
            "--no-underline" => style.underline = false,
            "--compact" => style.compact = true,
            "--no-compact" => style.compact = false,
            "--narrow" | "-n" => style.spacing = Some(0),
            "--wide" | "-w" => style.spacing = None,
            "--spacing" => style.spacing = Some(number(&value()?, 0, 128, flag)?),
            "--monospace" | "-m" => {
                style.monospace = true;
                style.monospace_width = None;
            }
            "--no-monospace" => {
                style.monospace = false;
                style.monospace_width = None;
            }
            "--monospace-width" => {
                style.monospace = true;
                style.monospace_width = Some(number(&value()?, 1, 512, flag)?);
            }
            "--color" | "-c" => style.color = Some(Color::parse(&value()?)?),
            "--no-color" => style.color = None,
            "--literal" => style.literal = true,
            "--no-literal" => style.literal = false,
            "--reset-style" => style = default.clone(),
            "--border" | "-b" => {
                options.top = true;
                options.bottom = true;
            }
            "--no-border" => {
                options.top = false;
                options.bottom = false;
            }
            "--top-border" | "--border-top" | "-t" => options.top = true,
            "--no-top-border" | "--no-border-top" => options.top = false,
            "--bottom-border" | "--border-bottom" => options.bottom = true,
            "--no-bottom-border" | "--no-border-bottom" => options.bottom = false,
            "--border-endcaps" => options.endcaps = true,
            "--no-border-endcaps" => options.endcaps = false,
            "--border-char" => {
                let c = value()?;
                font::check_cell(&c)?;
                options.top_char = Some(c.clone());
                options.bottom_char = Some(c.clone());
                options.left_endcap_char = Some(c.clone());
                options.right_endcap_char = Some(c);
            }
            "--endcap-char" => {
                let c = value()?;
                font::check_cell(&c)?;
                options.left_endcap_char = Some(c.clone());
                options.right_endcap_char = Some(c);
            }
            "--left-endcap-char" => {
                let c = value()?;
                font::check_cell(&c)?;
                options.left_endcap_char = Some(c);
            }
            "--right-endcap-char" => {
                let c = value()?;
                font::check_cell(&c)?;
                options.right_endcap_char = Some(c);
            }
            "--top-border-char" | "--border-top-char" => {
                let c = value()?;
                font::check_cell(&c)?;
                options.top_char = Some(c);
            }
            "--bottom-border-char" | "--border-bottom-char" => {
                let c = value()?;
                font::check_cell(&c)?;
                options.bottom_char = Some(c);
            }
            "--border-height" => options.border_height = Some(number(&value()?, 1, 16, flag)?),
            "--prefix" | "--line-prefix" | "--pre-line" => {
                options.prefix = value()?;
                font::check_art(&options.prefix)?;
            }
            "--suffix" | "--line-suffix" | "--post-line" => {
                options.suffix = value()?;
                font::check_art(&options.suffix)?;
            }
            "--slash-comment" | "--csharp-comment" | "--cs-comment" | "--cpp-comment"
            | "--c-comment" => wrappers(&mut options, "// ", ""),
            "--hash-comment"
            | "--powershell-comment"
            | "--shell-comment"
            | "--sh-comment"
            | "--python-comment" => wrappers(&mut options, "# ", ""),
            "--tick-comment" | "--apostrophe-comment" => wrappers(&mut options, "' ", ""),
            "--lua-comment" => wrappers(&mut options, "-- ", ""),
            "--block-comment"
            | "--slash-star-comment"
            | "--c-multi-line-comment"
            | "--cpp-multi-line-comment"
            | "--cs-multi-line-comment" => wrappers(&mut options, "/* ", " */"),
            "--html-comment" | "--xml-comment" | "--xsl-comment" => {
                wrappers(&mut options, "<!-- ", " -->")
            }
            "--no-comment" => wrappers(&mut options, "", ""),
            "--line-spacing" => options.line_spacing = number(&value()?, 0, 128, flag)?,
            "--tab-width" => options.tab_width = number(&value()?, 1, 128, flag)?,
            "--color-mode" => {
                options.color_mode = match value()?.as_str() {
                    "auto" => ColorMode::Auto,
                    "always" => ColorMode::Always,
                    "never" => ColorMode::Never,
                    _ => return Err("--color-mode must be auto, always, or never".into()),
                }
            }
            "--text-file" => inputs.push(Input {
                source: Source::File(value()?),
                style: style.clone(),
            }),
            "--stdin" => {
                if stdin_used {
                    return Err("--stdin can only be used once".into());
                }
                stdin_used = true;
                inputs.push(Input {
                    source: Source::Stdin,
                    style: style.clone(),
                });
            }
            "--newline" => {
                let mut s = style.clone();
                s.literal = true;
                inputs.push(Input {
                    source: Source::Text("\n".into()),
                    style: s,
                });
            }
            "--demo" => {
                let mut text = String::from(
                    "ABCDEFGHIJKLMNOPQRSTUVWXYZ\nabcdefghijklmnopqrstuvwxyz\n0123456789\n!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~\n",
                );
                let extra: Vec<_> = style
                    .font
                    .glyphs
                    .keys()
                    .filter(|c| !c.is_ascii())
                    .copied()
                    .collect();
                for chunk in extra.chunks(16) {
                    text.extend(chunk);
                    text.push('\n');
                }
                let mut s = style.clone();
                s.literal = true;
                inputs.push(Input {
                    source: Source::Text(text),
                    style: s,
                });
            }
            "--verbose" | "--debug" | "-v" => verbose = true,
            "--pause" | "-p" => pause = true,
            "--help" | "-h" => return Ok(Action::Help),
            "--version" => return Ok(Action::Version),
            "--list-fonts" | "--font-info" | "--export-font" | "--check-font" => {
                if action.is_some() {
                    return Err("choose one font utility per invocation".into());
                }
                action = Some(match flag {
                    "--list-fonts" => Action::List,
                    "--font-info" => Action::Info(value()?),
                    "--export-font" => Action::Export(value()?),
                    _ => Action::Check(value()?),
                });
            }
            _ => {
                return Err(format!(
                    "unknown option {flag:?}; use --help or -- before literal text"
                ));
            }
        }
    }
    if let Some(action) = action {
        if !inputs.is_empty() {
            return Err("font utilities cannot be combined with text input".into());
        }
        return Ok(action);
    }
    Ok(Action::Render(Box::new(Plan {
        inputs,
        final_style: style,
        options,
        verbose,
        pause,
    })))
}
#[derive(Clone, Copy, Debug, Default)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}
fn number(s: &str, min: usize, max: usize, flag: &str) -> Result<usize> {
    s.parse::<usize>()
        .ok()
        .filter(|v| (min..=max).contains(v))
        .ok_or_else(|| format!("{flag} requires an integer in {min}..{max}"))
}
fn wrappers(o: &mut Options, prefix: &str, suffix: &str) {
    o.prefix = prefix.into();
    o.suffix = suffix.into();
}
