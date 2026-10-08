use crate::{JsonNumber, Value};
use std::collections::HashMap;
use std::fmt;

/// An error produced while parsing JSON.
///
/// Every variant carries a byte position (`p`) pointing at where in the input
/// the error was detected. (Line/column conversion is a future enhancement.)
#[derive(Debug)]
pub enum ScanError {
    /// Encountered a character that isn't valid there.
    /// `c` is the offending byte as a `char`, `p` is its position.
    UnexpectedChar { c: char, p: usize },
    /// Reached the end of input before the value was complete
    /// (e.g. an unclosed string or array).
    UnexpectedEof,
    /// The input contained bytes that aren't valid UTF-8.
    InvalidUtf8 { p: usize },
    /// An invalid escape sequence inside a string (e.g. `\q`, or a lone
    /// surrogate in a `\uXXXX` escape).
    InvalidEscape { p: usize },
    /// A malformed number (e.g. `12.3.4`, or a non-finite value like `1e999`).
    InvalidNumber { p: usize },
    /// Non-whitespace content after the top-level value (e.g. `123abc`).
    TrailingData { p: usize },
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanError::UnexpectedChar { c, p } => write!(f, "unexpected '{c}' at byte {p}"),
            ScanError::UnexpectedEof => write!(f, "unexpected end of input"),
            ScanError::InvalidUtf8 { p } => write!(f, "invalid UTF-8 at byte {p}"),
            ScanError::InvalidEscape { p } => write!(f, "invalid escape at byte {p}"),
            ScanError::InvalidNumber { p } => write!(f, "invalid number at byte {p}"),
            ScanError::TrailingData { p } => write!(f, "trailing data at byte {p}"),
        }
    }
}

impl std::error::Error for ScanError {}

struct Scanner<'a> {
    i: &'a str,
    p: usize,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str) -> Self {
        Scanner { i: input, p: 0 }
    }

    fn peek(&self) -> Option<u8> {
        if self.p < self.i.len() {
            Some(self.i.as_bytes()[self.p])
        } else {
            None
        }
    }

    fn next(&mut self) -> Option<u8> {
        let val = self.peek()?;
        self.p += 1;
        Some(val)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.p += 1;
        }
    }

    fn expect(&mut self, expected: &[u8]) -> Result<(), ScanError> {
        for &byte in expected {
            match self.next() {
                Some(b) if b == byte => {}
                Some(b) => {
                    return Err(ScanError::UnexpectedChar {
                        c: b as char,
                        p: self.p,
                    });
                }
                None => return Err(ScanError::UnexpectedEof),
            }
        }
        Ok(())
    }

    fn parse_hex(&mut self) -> Result<u16, ScanError> {
        let mut val = 0u16;
        for _ in 0..4 {
            let b = self.next().ok_or(ScanError::UnexpectedEof)?;
            let digit = match b {
                b'0'..=b'9' => (b - b'0') as u16,
                b'a'..=b'f' => (b - b'a' + 10) as u16,
                b'A'..=b'F' => (b - b'A' + 10) as u16,
                _ => {
                    return Err(ScanError::InvalidEscape { p: self.p });
                }
            };
            val = (val << 4) | digit;
        }
        Ok(val)
    }

    fn parse_keyword(&mut self) -> Result<Value, ScanError> {
        match self.peek() {
            Some(b't') => {
                self.expect(b"true")?;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.expect(b"false")?;
                Ok(Value::Bool(false))
            }
            Some(b'n') => {
                self.expect(b"null")?;
                Ok(Value::Null)
            }
            None => Err(ScanError::UnexpectedEof),
            _ => unreachable!("parse_keyword only called for t/f/n"),
        }
    }

    fn parse_string(&mut self) -> Result<Value, ScanError> {
        let mut str_vec: Vec<u8> = Vec::new();
        self.next();

        loop {
            match self.next() {
                Some(b'"') => break,
                Some(b'\\') => match self.next() {
                    Some(b'"') => str_vec.push(b'"'),
                    Some(b'\\') => str_vec.push(b'\\'),
                    Some(b'/') => str_vec.push(b'/'),
                    Some(b'n') => str_vec.push(b'\n'),
                    Some(b't') => str_vec.push(b'\t'),
                    Some(b'r') => str_vec.push(b'\r'),
                    Some(b'b') => str_vec.push(0x08),
                    Some(b'f') => str_vec.push(0x0c),
                    Some(b'u') => {
                        let c1 = self.parse_hex()?;
                        let cp = if (0xD800..=0xDBFF).contains(&c1) {
                            self.expect(b"\\u")?;
                            let c2 = self.parse_hex()?;
                            if !(0xDC00..=0xDFFF).contains(&c2) {
                                return Err(ScanError::InvalidEscape { p: self.p });
                            }
                            0x10000 + ((c1 as u32 - 0xD800) << 10) + (c2 as u32 - 0xDC00)
                        } else {
                            c1 as u32
                        };
                        let ch =
                            char::from_u32(cp).ok_or(ScanError::InvalidEscape { p: self.p })?;
                        let mut buffer = [0u8; 4];
                        let s = ch.encode_utf8(&mut buffer);
                        str_vec.extend_from_slice(s.as_bytes());
                    }
                    Some(_) => {
                        return Err(ScanError::InvalidEscape { p: self.p });
                    }
                    None => return Err(ScanError::UnexpectedEof),
                },
                Some(c) => str_vec.push(c),
                None => return Err(ScanError::UnexpectedEof),
            }
        }
        let s = String::from_utf8(str_vec).map_err(|_| ScanError::InvalidUtf8 { p: self.p })?;
        Ok(Value::Str(s))
    }

    fn parse_number(&mut self) -> Result<Value, ScanError> {
        let start = self.p;
        while matches!(
            self.peek(),
            Some(b'0'..=b'9' | b'-' | b'.' | b'e' | b'E' | b'+')
        ) {
            self.next();
        }
        let text = &self.i[start..self.p];
        let num = if text.contains('.') || text.contains('e') || text.contains('E') {
            let num = text
                .parse::<f64>()
                .map_err(|_| ScanError::InvalidNumber { p: self.p })?;
            if !num.is_finite() {
                return Err(ScanError::InvalidNumber { p: self.p });
            }
            JsonNumber::Float(num)
        } else if text.starts_with('-') {
            text.parse::<i64>()
                .map(JsonNumber::Int)
                .map_err(|_| ScanError::InvalidNumber { p: self.p })?
        } else {
            text.parse::<u64>()
                .map(JsonNumber::UInt)
                .map_err(|_| ScanError::InvalidNumber { p: self.p })?
        };

        Ok(Value::Number(num))
    }

    fn parse_array(&mut self) -> Result<Value, ScanError> {
        self.next();
        self.skip_whitespace();

        if matches!(self.peek(), Some(b']')) {
            self.next();
            return Ok(Value::Array(Vec::new()));
        }

        let mut val = Vec::new();

        loop {
            self.skip_whitespace();
            let e = self.parse_value()?;
            val.push(e);
            self.skip_whitespace();

            match self.peek() {
                Some(b',') => {
                    self.next();
                }
                Some(b']') => {
                    self.next();
                    break;
                }
                Some(c) => {
                    return Err(ScanError::UnexpectedChar {
                        c: c as char,
                        p: self.p,
                    });
                }
                None => return Err(ScanError::UnexpectedEof),
            }
        }

        Ok(Value::Array(val))
    }

    fn parse_object(&mut self) -> Result<Value, ScanError> {
        self.next();
        self.skip_whitespace();

        if matches!(self.peek(), Some(b'}')) {
            self.next();
            return Ok(Value::Object(HashMap::new()));
        }

        let mut obj = HashMap::new();

        loop {
            self.skip_whitespace();

            let key = match self.parse_string()? {
                Value::Str(s) => s,
                _ => unreachable!(),
            };

            self.skip_whitespace();
            self.expect(b":")?;
            self.skip_whitespace();

            let val = self.parse_value()?;

            obj.insert(key, val);

            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.next();
                }
                Some(b'}') => {
                    self.next();
                    break;
                }
                Some(c) => {
                    return Err(ScanError::UnexpectedChar {
                        c: c as char,
                        p: self.p,
                    });
                }
                None => return Err(ScanError::UnexpectedEof),
            }
        }

        Ok(Value::Object(obj))
    }

    fn parse_value(&mut self) -> Result<Value, ScanError> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string(),
            Some(b't') | Some(b'f') | Some(b'n') => self.parse_keyword(),
            Some(b'0'..=b'9') | Some(b'-') => self.parse_number(),
            Some(c) => Err(ScanError::UnexpectedChar {
                c: c as char,
                p: self.p,
            }),
            None => Err(ScanError::UnexpectedEof),
        }
    }
}

/// Parses a JSON document into a [`Value`].
///
/// Walks the entire input and builds the value tree, supporting every JSON
/// type (objects, arrays, strings with escapes and surrogate pairs, numbers,
/// booleans, null). Returns [`ScanError`] if the input is malformed, contains
/// an invalid escape or number, or has trailing data after the value.
///
/// # Example
///
/// ```
/// use rsonist::to_value;
///
/// let value = to_value(r#"{"name": "alice", "age": 30}"#)?;
/// assert_eq!(value.get("name").unwrap().as_str().unwrap(), "alice");
/// # Ok::<(), rsonist::ScanError>(())
/// ```
pub fn to_value(json: &str) -> Result<Value, ScanError> {
    let mut scanner = Scanner::new(json);
    let object = scanner.parse_value()?;
    scanner.skip_whitespace();
    match scanner.peek() {
        Some(_) => Err(ScanError::TrailingData { p: scanner.p }),
        None => Ok(object),
    }
}
