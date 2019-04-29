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
    EmptyName,
}

/// https://drafts.css-houdini.org/css-properties-values-api-1/#multipliers
#[derive(Debug, PartialEq)]
pub enum Multiplier {
    Space,
    Comma,
}

#[derive(Debug, PartialEq)]
pub struct Component {
    pub name: ComponentName,
    pub multiplier: Option<Multiplier>,
}

#[derive(Debug, PartialEq)]
pub struct CustomIdent(Box<[u8]>);

impl CustomIdent {
    fn from_bytes(ident: &[u8]) -> Result<Self, ParseError> {
        if ident.eq_ignore_ascii_case(b"inherit") ||
            ident.eq_ignore_ascii_case(b"reset") ||
            ident.eq_ignore_ascii_case(b"revert") ||
            ident.eq_ignore_ascii_case(b"unset") ||
            ident.eq_ignore_ascii_case(b"default") {
            return Err(ParseError::InvalidCustomIdent);
        }
        Ok(CustomIdent(ident.to_vec().into_boxed_slice()))
    }
}


#[derive(Debug, PartialEq)]
pub enum ComponentName {
    DataType(DataType),
    Ident(CustomIdent),
}

impl DataType {
    fn is_pre_multiplied(&self) -> bool {
        false
    }
}

impl ComponentName {
    /// https://drafts.css-houdini.org/css-properties-values-api-1/#pre-multiplied-data-type-name
    fn is_pre_multiplied(&self) -> bool {
        match *self {
            ComponentName::DataType(ref t) => t.is_pre_multiplied(),
            ComponentName::Ident(..) => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DataType {}

/// Parse a syntax descriptor or universal syntax descriptor.
pub fn parse_descriptor(input: &str) -> Result<Descriptor, ParseError> {
    let input = input.as_bytes();
    // 1. Strip leading and trailing ASCII whitespace from string.
    let input = ascii::trim_ascii_whitespace(input);

    // 2. If string's length is 0, return failure.
    if input.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    // 3. If string's length is 1, and the only code point in string is U+002A
    //    ASTERISK (*), return the universal syntax descriptor.
    if input.len() == 1 && input[0] == b'*' {
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
    input: &'a [u8],
    position: usize,
    output: &'b mut Vec<Component>,
}

/// https://drafts.csswg.org/css-syntax-3/#whitespace
fn is_whitespace(byte: u8) -> bool {
    match byte {
        b'\t' | b'\n' | b'\r' | b'\x0c' => true,
        _ => false,
    }
}

/// https://drafts.csswg.org/css-syntax-3/#letter
fn is_letter(byte: u8) -> bool {
    match byte {
        b'A'...b'Z' |
        b'a'...b'a' => true,
        _ => false,
    }
}

/// https://drafts.csswg.org/css-syntax-3/#non-ascii-code-point
fn is_non_ascii(byte: u8) -> bool {
    byte >= 0x80
}

/// https://drafts.csswg.org/css-syntax-3/#digit
fn is_digit(byte: u8) -> bool {
    match byte {
        b'0'...b'9' => true,
        _ => false,
    }
}

/// https://drafts.csswg.org/css-syntax-3/#name-start-code-point
fn is_name_start(byte: u8) -> bool {
    is_letter(byte) || is_non_ascii(byte) || byte == b'_'
}

/// https://drafts.csswg.org/css-syntax-3/#name-code-point
fn is_name(byte: u8) -> bool {
    is_name_start(byte) || is_digit(byte) || byte == b'-'
}

impl<'a, 'b> Parser<'a, 'b> {
    fn new(input: &'a [u8], output: &'b mut Vec<Component>) -> Self {
        Self {
            input,
            position: 0,
            output,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.position).cloned()
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
        unimplemented!()
    }

    /// https://drafts.csswg.org/css-syntax-3/#consume-a-name
    /// FIXME(emilio): This should actually use cssparser's consume_name
    /// to handle correctly escaping and nulls.
    fn consume_name(&mut self) -> &'a [u8] {
        let start = self.position;

        loop {
            let byte = match self.peek() {
                None => return &self.input[start..],
                Some(b) => b,
            };

            if !is_name(byte) {
                break;
            }
            self.position += 1;
        }

        &self.input[start..self.position]
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

        let name = self.consume_name();
        if name.is_empty() {
            return Err(ParseError::EmptyName);
        }
        return Ok(ComponentName::Ident(CustomIdent::from_bytes(name)?))
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
