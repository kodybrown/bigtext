use crate::Result;

pub fn decode(text: &str) -> Result<String> {
    let mut out = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let fail = |reason: &str| {
            format!(
                "escape at byte {}: {reason}; use --literal for unescaped text",
                offset + 1
            )
        };
        let (_, kind) = chars.next().ok_or_else(|| fail("trailing backslash"))?;
        match kind {
            '\\' => out.push('\\'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' | 'U' => {
                let braced = kind == 'u' && chars.peek().is_some_and(|(_, c)| *c == '{');
                let mut digits = String::new();
                if braced {
                    chars.next();
                    loop {
                        let (_, c) = chars
                            .next()
                            .ok_or_else(|| fail("unclosed Unicode escape"))?;
                        if c == '}' {
                            break;
                        }
                        if !c.is_ascii_hexdigit() || digits.len() >= 6 {
                            return Err(fail("expected 1..6 hexadecimal digits inside braces"));
                        }
                        digits.push(c);
                    }
                } else {
                    for _ in 0..if kind == 'u' { 4 } else { 8 } {
                        let (_, c) = chars
                            .next()
                            .ok_or_else(|| fail("incomplete Unicode escape"))?;
                        if !c.is_ascii_hexdigit() {
                            return Err(fail("invalid hexadecimal digit"));
                        }
                        digits.push(c);
                    }
                }
                let n = u32::from_str_radix(&digits, 16)
                    .map_err(|_| fail("empty or invalid Unicode escape"))?;
                out.push(char::from_u32(n).ok_or_else(|| fail("not a Unicode scalar value"))?);
            }
            _ => return Err(fail(&format!("unknown escape \\{kind}"))),
        }
    }
    Ok(out)
}
