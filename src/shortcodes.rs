use anyhow::Result;

pub struct Shortcode {
    pub tag: &'static str,
    pub render: fn(&[String]) -> String,
}

pub fn expand_shortcodes(input: &str, handlers: &[Shortcode]) -> String {
    let mut output = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\\' {
            // look for a backslash introducing a shortcode
            let start = i;
            let mut matched = None;

            for handler in handlers {
                let tag_chars: Vec<char> = handler.tag.chars().collect();
                if chars[start + 1..].starts_with(&tag_chars) {
                    matched = Some(handler);
                    break;
                }
            }

            if let Some(handler) = matched {
                // parse args
                let mut args = Vec::new();
                let mut pos = i + 1 + handler.tag.chars().count();
                while pos < chars.len() && chars[pos] == '{' {
                    let (arg, new_pos) = parse_braced(&chars, pos).unwrap();
                    args.push(arg);
                    pos = new_pos;
                }

                output.push_str(&(handler.render)(&args));
                i = pos;
                continue;
            }
        }

        output.push(chars[i]);
        i += 1;
    }

    output
}

fn parse_braced(input: &[char], start: usize) -> Result<(String, usize)> {
    if input.get(start) != Some(&'{') {
        return Err(anyhow::anyhow!("Expected opening brace"));
    }

    let mut depth = 0;
    let mut content = String::new();

    for (i, &c) in input[start..].iter().enumerate() {
        match c {
            '{' if depth == 0 => depth = 1, // first opening brace, don’t push
            '{' => {
                depth += 1;
                content.push(c);
            }
            '}' if depth == 1 => return Ok((content, start + i + 1)),
            '}' if depth > 1 => {
                depth -= 1;
                content.push(c);
            }
            _ => content.push(c),
        }
    }

    Err(anyhow::anyhow!("Unbalanced braces"))
}
