use nom::{
    bytes::complete::tag as nom_tag,
    bytes::complete::take_until,
    bytes::streaming::take_while1,
    character::complete::multispace0,
    error::Error,
    multi::many0,
    sequence::{delimited, terminated},
    IResult,
};

pub trait Shortcode {
    /// Construct an "empty" shortcode of this type.
    fn empty() -> Self;

    /// Return the tag name (e.g. "note").
    fn tag(&self) -> String;

    /// Construct a shortcode from argument slices.
    fn mkvalue(args: &[String]) -> Self
    where
        Self: Sized;

    /// Render the shortcode as a string (usually HTML).
    fn render_shortcode(&self, content: &str) -> String;
}

pub struct SomeShortcode {
    pub tag: String,
    pub parser: fn(&[String], String) -> String,
}

impl SomeShortcode {
    pub fn new<S>() -> Self
    where
        S: Shortcode,
    {
        SomeShortcode {
            tag: S::empty().tag(),
            parser: |args, content| {
                let inst = S::mkvalue(args);
                inst.render_shortcode(&content)
            },
        }
    }
}

fn parse_attributes(input: &str) -> IResult<&str, Vec<&str>> {
    many0(terminated(parse_attribute, multispace0))(input)
}

fn parse_attribute(input: &str) -> IResult<&str, &str, Error<&str>> {
    let (input, _key) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;

    // allow optional whitespace before '='
    let (input, _) = multispace0(input)?;
    let (input, _) = nom_tag("=")(input)?;

    // allow optional whitespace before '='
    let (input, _) = multispace0(input)?;

    let (input, value) = if let Ok((input, val)) = delimited(
        nom_tag::<&str, &str, Error<&str>>("\""),
        take_until("\""),
        nom_tag::<&str, &str, Error<&str>>("\""),
    )(input)
    {
        (input, val)
    } else {
        take_while1(|c: char| !c.is_whitespace() && c != ']')(input)?
    };
    Ok((input, value))
}

fn parse_shortcode<'a>(input: &'a str, handler: &'a SomeShortcode) -> IResult<&'a str, String> {
    let (input, _) = nom_tag("[")(input)?;
    let (input, _) = nom_tag(handler.tag.as_str())(input)?;
    let (input, _) = multispace0(input)?;

    // parse attributes as key=value pairs
    let (input, attrs) = parse_attributes(input)?;
    let (input, _) = nom_tag("]")(input)?;

    // parse content until closing tag
    let closing_tag = format!("[/{}]", handler.tag);
    let (input, content) = take_until(&*closing_tag)(input)?;
    let (input, _) = nom_tag(&*closing_tag)(input)?;
    let aaaa = attrs.into_iter().map(|s| s.to_string()).collect::<Vec<_>>();
    // call the shortcode parser with parsed attrs and inner content
    Ok((input, (handler.parser)(&aaaa, content.to_string())))
}

pub fn expand_shortcodes(input: &str, handlers: &[SomeShortcode]) -> String {
    let mut output = String::new();
    let mut rest = input.to_string();

    while !rest.is_empty() {
        let mut matched = false;

        for handler in handlers {
            if let Some(_stripped) = rest.strip_prefix(&format!("[{}", handler.tag)) {
                // put back the '[' we stripped
                //let parse_input = &rest[1..];
                if let Ok((new_rest, expanded)) = parse_shortcode(&rest, handler) {
                    output.push_str(&expanded);
                    rest = new_rest.to_string();
                    matched = true;
                    break;
                }
            }
        }

        if !matched {
            // copy first char to output
            output.push(rest.chars().next().unwrap());
            rest = rest.chars().skip(1).collect(); //&rest[1..];
        }
    }
    output
}
