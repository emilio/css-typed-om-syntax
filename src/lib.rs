use std::borrow::Cow;

mod ascii;

/// https://drafts.css-houdini.org/css-properties-values-api-1/#parsing-syntax
#[derive(Debug, PartialEq)]
pub struct Descriptor(Box<[Component]>);
impl Descriptor {
    fn universal() -> Self {
        Descriptor(Box::new([]))
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    EmptyInput,
    UnexpectedEOF,
    UnexpectedPipe,
    InvalidCustomIdent,
    InvalidNameStart,
    InvalidName,
    UnclosedDataTypeName,
    UnknownDataTypeName,
}

/// https://drafts.css-houdini.org/css-properties-values-api-1/#multipliers
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Multiplier {
    Space,
    Comma,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    name: ComponentName,
    multiplier: Option<Multiplier>,
}

impl Component {
    #[inline]
    pub fn name(&self) -> &ComponentName {
        &self.name
    }

    #[inline]
    pub fn multiplier(&self) -> Option<Multiplier> {
        self.multiplier
    }

    #[inline]
    pub fn unpremultipied(&self) -> Cow<Self> {
        match self.name.unpremultiply() {
            Some(component) => {
                debug_assert!(
                    self.multiplier.is_none(),
                    "Shouldn't have parsed a multiplier for a pre-multiplied data type name",
                );
                Cow::Owned(component)
            }
            None => Cow::Borrowed(self),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomIdent(Box<str>);

impl CustomIdent {
    fn from_ident(ident: &str) -> Result<Self, ()> {
        if ident.eq_ignore_ascii_case("inherit") ||
            ident.eq_ignore_ascii_case("reset") ||
            ident.eq_ignore_ascii_case("revert") ||
            ident.eq_ignore_ascii_case("unset") ||
            ident.eq_ignore_ascii_case("default") {
            return Err(());
        }
        Ok(CustomIdent(ident.to_owned().into_boxed_str()))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ComponentName {
    DataType(DataType),
    Ident(CustomIdent),
}

impl ComponentName {
    fn unpremultiply(&self) -> Option<Component> {
        match *self {
            ComponentName::DataType(ref t) => t.unpremultiply(),
            ComponentName::Ident(..) => None,
        }
    }

    /// https://drafts.css-houdini.org/css-properties-values-api-1/#pre-multiplied-data-type-name
    fn is_pre_multiplied(&self) -> bool {
        self.unpremultiply().is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataType {
    Length,
    Number,
    Percentage,
    LengthPercentage,
    Color,
    Image,
    Url,
    Integer,
    Angle,
    Time,
    Resolution,
    TransformFunction,
    TransformList,
    CustomIdent,
}

impl DataType {
    fn unpremultiply(&self) -> Option<Component> {
        match *self {
            DataType::TransformList => Some(Component {
                name: ComponentName::DataType(DataType::TransformFunction),
                multiplier: Some(Multiplier::Space),
            }),
            _ => None,
        }
    }
}

impl DataType {
    fn from_bytes(ty: &[u8]) -> Option<Self> {
        Some(match ty {
            b"length" => DataType::Length,
            b"number" => DataType::Number,
            b"percentage" => DataType::Percentage,
            b"length-percentage" => DataType::LengthPercentage,
            b"color" => DataType::Color,
            b"image" => DataType::Image,
            b"url" => DataType::Url,
            b"integer" => DataType::Integer,
            b"angle" => DataType::Angle,
            b"time" => DataType::Time,
            b"resolution" => DataType::Resolution,
            b"transform-function" => DataType::TransformFunction,
            b"custom-ident" => DataType::CustomIdent,
            b"transform-list" => DataType::TransformList,
            _ => return None,
        })
    }
}

/// Parse a syntax descriptor or universal syntax descriptor.
pub fn parse_descriptor(input: &str) -> Result<Descriptor, ParseError> {
    // 1. Strip leading and trailing ASCII whitespace from string.
    let input = ascii::trim_ascii_whitespace(input);

    // 2. If string's length is 0, return failure.
    if input.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    // 3. If string's length is 1, and the only code point in string is U+002A
    //    ASTERISK (*), return the universal syntax descriptor.
    if input.len() == 1 && input.as_bytes()[0] == b'*' {
        return Ok(Descriptor::universal());
    }

    // 4. Let stream be an input stream created from the code points of string,
    //    preprocessed as specified in [css-syntax-3]. Let descriptor be an
    //    initially empty list of syntax components.
    //
    // NOTE(emilio): Instead of preprocessing we cheat and treat new-lines and
    // nulls in the parser specially.
    let mut components = vec![];
    {
        let mut parser = Parser::new(input, &mut components);
        // 5. Repeatedly consume the next input code point from stream.
        parser.parse()?;
    }
    Ok(Descriptor(components.into_boxed_slice()))
}

struct Parser<'a, 'b> {
    input: &'a str,
    position: usize,
    output: &'b mut Vec<Component>,
}

/// https://drafts.csswg.org/css-syntax-3/#whitespace
fn is_whitespace(byte: u8) -> bool {
    match byte {
        b'\t' | b'\n' | b'\r' | b' ' => true,
        _ => false,
    }
}

/// https://drafts.csswg.org/css-syntax-3/#letter
fn is_letter(byte: u8) -> bool {
    match byte {
        b'A'...b'Z' |
        b'a'...b'z' => true,
        _ => false,
    }
}

/// https://drafts.csswg.org/css-syntax-3/#non-ascii-code-point
fn is_non_ascii(byte: u8) -> bool {
    byte >= 0x80
}

/// https://drafts.csswg.org/css-syntax-3/#name-start-code-point
fn is_name_start(byte: u8) -> bool {
    is_letter(byte) || is_non_ascii(byte) || byte == b'_'
}

impl<'a, 'b> Parser<'a, 'b> {
    fn new(input: &'a str, output: &'b mut Vec<Component>) -> Self {
        Self {
            input,
            position: 0,
            output,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).cloned()
    }

    fn parse(&mut self) -> Result<(), ParseError> {
        // 5. Repeatedly consume the next input code point from stream:
        loop {
            let byte = match self.peek() {
                None => {
                    // EOF: If descriptor's size is greater than zero, return
                    // descriptor; otherwise, return failure.
                    if self.output.is_empty() {
                        return Err(ParseError::UnexpectedEOF);
                    }
                    return Ok(());
                }
                Some(b) => b,
            };

            // whitespace: Do nothing.
            if is_whitespace(byte) {
                self.position += 1;
                continue;
            }

            // U+007C VERTICAL LINE (|):
            //  * If descriptor's size is greater than zero, consume a syntax
            //    component from stream. If failure was returned, return failure;
            //    otherwise, append the returned value to descriptor.
            //  * If descriptor's size is zero, return failure.
            if byte == b'|' {
                if self.output.is_empty() {
                    return Err(ParseError::UnexpectedPipe);
                }
                self.position += 1;
            }

            let component = self.parse_component()?;
            self.output.push(component)
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some(c) if is_whitespace(c) => self.position += 1,
                _ => return,
            }
        }
    }

    /// https://drafts.css-houdini.org/css-properties-values-api-1/#consume-data-type-name
    fn parse_data_type_name(&mut self) -> Result<DataType, ParseError> {
        let start = self.position;
        loop {
            let byte = match self.peek() {
                Some(b) => b,
                None => return Err(ParseError::UnclosedDataTypeName),
            };
            if byte != b'>' {
                self.position += 1;
                continue;
            }
            let ty = match DataType::from_bytes(&self.input.as_bytes()[start..self.position]) {
                Some(ty) => ty,
                None => return Err(ParseError::UnknownDataTypeName),
            };
            self.position += 1;
            return Ok(ty)
        }
    }

    fn parse_name(&mut self) -> Result<ComponentName, ParseError> {
        let b = match self.peek() {
            Some(b) => b,
            None => return Err(ParseError::UnexpectedEOF),
        };

        if b == b'<' {
            self.position += 1;
            return Ok(ComponentName::DataType(self.parse_data_type_name()?));
        }

        if b != b'\\' && !is_name_start(b) {
            return Err(ParseError::InvalidNameStart);
        }

        let input = &self.input[self.position..];
        let mut input = cssparser::ParserInput::new(input);
        let mut input = cssparser::Parser::new(&mut input);
        let name = input
            .expect_ident()
            .map_err(|_| ())
            .and_then(|name| CustomIdent::from_ident(name.as_ref()));
        let name = match name {
            Ok(name) => name,
            Err(..) => return Err(ParseError::InvalidName),
        };
        self.position += input.position().byte_index();
        return Ok(ComponentName::Ident(name))
    }

    fn parse_multiplier(&mut self) -> Option<Multiplier> {
        let multiplier = match self.peek()? {
            b'+' => Multiplier::Space,
            b'#' => Multiplier::Comma,
            _ => return None,
        };
        self.position += 1;
        Some(multiplier)
    }

    /// https://drafts.css-houdini.org/css-properties-values-api-1/#consume-a-syntax-component
    fn parse_component(&mut self) -> Result<Component, ParseError> {
        // Consume as much whitespace as possible from stream.
        self.skip_whitespace();
        let name = self.parse_name()?;
        let multiplier = if name.is_pre_multiplied() {
            None
        } else {
            self.parse_multiplier()
        };
        Ok(Component { name, multiplier })
    }
}

#[test]
fn universal() {
    for syntax in &["*", " * ", "* ", "\t*\t"] {
        assert_eq!(parse_descriptor(syntax), Ok(Descriptor::universal()));
    }
}

#[test]
fn simple_length() {
    macro_rules! ident {
        ($str:expr) => {
            ComponentName::Ident(CustomIdent::from_ident($str).unwrap())
        }
    }
    assert_eq!(parse_descriptor("foo <length>#"), Ok(Descriptor(Box::new([
        Component {
            name: ident!("foo"),
            multiplier: None,
        },
        Component {
            name: ComponentName::DataType(DataType::Length),
            multiplier: Some(Multiplier::Comma),
        },
    ]))))
}
