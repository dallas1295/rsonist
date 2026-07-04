use std::collections::BTreeMap;

/// Build a `Value` using JSON-like literal syntax.
///
/// ```
/// use rsonist::rson;
///
/// let v = rson!({
///     "name": "alice",
///     "age": 30,
///     "active": true,
///     "scores": rson!([10, 20, 30]),
///     "meta": rson!({ "role": "admin" }),
/// });
/// ```
#[macro_export]
macro_rules! rson {
    // Object: { "k": v, "k2": v2 }
    ( { $($key:tt : $val:expr),* $(,)? } ) => {{
        let mut b = $crate::JSONBuilder::new();
        $( b.add_pair($key, $val); )*
        b.build_val()
    }};

    // Array: [ v, v, v ] — each element is .into()'d so mixed types collapse to Vec<Value>
    ( [ $($val:expr),* $(,)? ] ) => {{
        $crate::Value::Array(vec![ $( ($val).into() ),* ])
    }};

    // Bare scalar: 42, "hi", true, my_string_var, ...
    ( $val:expr ) => {
        ::std::convert::Into::<$crate::Value>::into($val)
    };
}

/// A JSON number, distinguishing unsigned integers, signed integers, and floats.
///
/// You usually don't construct this directly — it's produced by [`to_value`]
/// (which picks the variant based on the JSON text) or by the `From` impls on
/// `u64`, `i64`, and `f64`. To pull a concrete number out of a [`Value`], use
/// [`Value::as_i64`], [`Value::as_u64`], or [`Value::as_f64`] instead of
/// matching on this enum directly.
///
/// [`to_value`]: crate::to_value
pub enum JsonNumber {
    /// A non-negative integer parsed from JSON (e.g. `42`).
    UInt(u64),
    /// A negative integer parsed from JSON (e.g. `-7`).
    Int(i64),
    /// A floating-point number parsed from JSON (e.g. `3.14`, `1e10`).
    Float(f64),
}
impl From<u64> for JsonNumber {
    fn from(n: u64) -> Self {
        JsonNumber::UInt(n)
    }
}

impl From<i64> for JsonNumber {
    fn from(n: i64) -> Self {
        JsonNumber::Int(n)
    }
}

impl From<f64> for JsonNumber {
    fn from(n: f64) -> Self {
        JsonNumber::Float(n)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}

impl From<&String> for Value {
    fn from(s: &String) -> Self {
        Value::Str(s.to_string())
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}

/// Any JSON value.
///
/// This is the central type of rsonist: every JSON document parses into a
/// `Value`, and every document is serialized from one. The variants map
/// one-to-one onto the JSON data types.
///
/// Navigate into nested values with [`get`](Value::get) and
/// [`get_from`](Value::get_from), then extract a primitive with an `as_*`
/// method. Construct one with [`JSONBuilder`], the `From` impls, or by hand.
///
/// [`JSONBuilder`]: crate::JSONBuilder
pub enum Value {
    /// JSON `null`.
    Null,
    /// JSON `true` or `false`.
    Bool(bool),
    /// A JSON number. See [`JsonNumber`].
    Number(JsonNumber),
    /// A JSON string.
    Str(String),
    /// A JSON array. Elements are themselves [`Value`]s.
    Array(Vec<Value>),
    /// A JSON object. Keys are strings, values are [`Value`]s.
    ///
    /// Backed by a [`BTreeMap`], so keys serialize in sorted order (deterministic
    /// output) rather than insertion order.
    Object(BTreeMap<String, Value>),
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i8> for Value {
    fn from(n: i8) -> Self {
        Value::Number(JsonNumber::Int(n.into()))
    }
}

impl From<u8> for Value {
    fn from(n: u8) -> Self {
        Value::Number(JsonNumber::UInt(n.into()))
    }
}

impl From<i16> for Value {
    fn from(n: i16) -> Self {
        Value::Number(JsonNumber::Int(n.into()))
    }
}

impl From<u16> for Value {
    fn from(n: u16) -> Self {
        Value::Number(JsonNumber::UInt(n.into()))
    }
}

impl From<u32> for Value {
    fn from(n: u32) -> Self {
        Value::Number(JsonNumber::UInt(n.into()))
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(JsonNumber::Int(n.into()))
    }
}

impl From<f32> for Value {
    fn from(n: f32) -> Self {
        Value::Number(JsonNumber::Float(n.into()))
    }
}

impl From<u64> for Value {
    fn from(n: u64) -> Self {
        Value::Number(JsonNumber::UInt(n))
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(JsonNumber::Int(n))
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(JsonNumber::Float(n))
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<BTreeMap<String, T>> for Value {
    fn from(map: BTreeMap<String, T>) -> Self {
        Value::Object(map.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

impl Value {
    /// Serializes this value back to JSON text.
    ///
    /// The inverse of [`to_value`](crate::to_value): parses → `Value`
    /// → `to_str` round-trips for finite numbers and supported escapes.
    /// Non-finite floats (infinity, NaN) are emitted as `null`, matching the
    /// JSON spec's lack of non-finite number representations.
    pub fn to_str(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => {
                let mut out = String::from("\"");
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\t' => out.push_str("\\t"),
                        '\u{8}' => out.push_str("\\b"),
                        '\u{c}' => out.push_str("\\f"),
                        c if c <= '\u{1f}' => out.push_str(&format!("\\u{:04x}", c as u32)),
                        c => out.push(c),
                    }
                }
                out.push('"');
                out
            }
            Value::Number(n) => match n {
                JsonNumber::UInt(u) => u.to_string(),
                JsonNumber::Int(i) => i.to_string(),
                JsonNumber::Float(f) => {
                    if f.is_finite() {
                        f.to_string()
                    } else {
                        "null".to_string()
                    }
                }
            },
            Value::Array(i) => {
                // this turns the Array into an iter and then recursively serializes remaining children
                let arr: Vec<String> = i.iter().map(|i| i.to_str()).collect();
                format!("[{}]", arr.join(" , "))
            }
            Value::Object(m) => {
                // this turns the BTreeMap into an iter and then recursively serializes remaining children
                let obj: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, v.to_str()))
                    .collect();
                format!("{{{}}}", obj.join(", "))
            }
        }
    }

    /// Looks up a value by key, if this is an object.
    ///
    /// Returns `None` if this value isn't an object, or if the key is absent.
    /// Chainable with `?` to drill into nested structures:
    ///
    /// ```
    /// use rsonist::to_value;
    /// let root = to_value(r#"{"a": {"b": 1}}"#).unwrap();
    /// let b = root.get("a").unwrap().get("b").unwrap().as_i64().unwrap();
    /// assert_eq!(b, 1);
    /// ```
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(m) => m.get(key),
            _ => None,
        }
    }

    /// Looks up a value by index, if this is an array.
    ///
    /// Returns `None` if this value isn't an array, or if the index is out of
    /// bounds. Combine with [`get`](Value::get) to reach into arrays nested
    /// inside objects:
    ///
    /// ```
    /// use rsonist::to_value;
    /// let root = to_value(r#"{"items": ["x", "y", "z"]}"#).unwrap();
    /// let second = root.get("items").unwrap().get_from(1).unwrap().as_str().unwrap();
    /// assert_eq!(second, "y");
    /// ```
    pub fn get_from(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Array(arr) => arr.get(index),
            _ => None,
        }
    }

    /// Returns the string if this is a [`Value::Str`], else `None`.
    ///
    /// Borrows from the value — no allocation. Call `.to_string()` on the
    /// result if you need an owned `String`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the boolean if this is a [`Value::Bool`], else `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the value as `i64`, if it's an integer that fits.
    ///
    /// Works on both [`JsonNumber::Int`] and [`JsonNumber::UInt`] (widening
    /// the unsigned value when it fits in `i64`). Returns `None` for floats
    /// or unsigned values exceeding `i64::MAX`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(JsonNumber::Int(n)) => Some(*n),
            Value::Number(JsonNumber::UInt(n)) => i64::try_from(*n).ok(),
            _ => None,
        }
    }

    /// Returns the value as `u64`, if it's a non-negative integer.
    ///
    /// Works on [`JsonNumber::UInt`] directly, and on [`JsonNumber::Int`]
    /// when the value is `>= 0`. Returns `None` for negatives or floats.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(JsonNumber::UInt(n)) => Some(*n),
            Value::Number(JsonNumber::Int(n)) => {
                if *n >= 0 {
                    Some(*n as u64)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Returns the value as `f64`, for any number variant.
    ///
    /// This is the universal numeric escape hatch — integers widen to `f64`
    /// losslessly for all values up to 2^53, and floats pass through directly.
    /// Returns `None` only for non-numeric values.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(JsonNumber::Float(f)) => Some(*f),
            Value::Number(JsonNumber::Int(n)) => Some(*n as f64),
            Value::Number(JsonNumber::UInt(n)) => Some(*n as f64),
            _ => None,
        }
    }

    /// Returns the object map if this is a [`Value::Object`], else `None`.
    ///
    /// Borrows the inner [`BTreeMap`] — iterate entries directly without
    /// allocating. Chain with [`get`](Value::get) to drill in:
    ///
    /// ```
    /// use rsonist::to_value;
    /// let root = to_value(r#"{"a": 1, "b": 2}"#).unwrap();
    /// if let Some(obj) = root.as_obj() {
    ///     for (key, val) in obj {
    ///         println!("{key}: {}", val.to_str());
    ///     }
    /// }
    /// ```
    pub fn as_obj(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the array slice if this is a [`Value::Array`], else `None`.
    ///
    /// Borrows the inner elements as a slice — iterate directly without
    /// allocating.
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Serializes this value to pretty-printed (indented, multi-line) JSON text.
    ///
    /// Unlike [`to_str`](Value::to_str), which emits compact single-line JSON,
    /// `prettify` inserts a newline after every `{`, `[`, and `,`, and indents
    /// each nested level by two spaces. Intended for output read by humans.
    ///
    /// # Example
    ///
    /// ```
    /// use rsonist::JSONBuilder;
    ///
    /// let mut builder = JSONBuilder::new();
    /// builder.add_pair("name", "alice".to_string());
    /// builder.add_pair("scores", vec![10, 20, 30]);
    /// let pretty = builder.build_val().prettify();
    /// ```
    ///
    /// Non-finite floats (infinity, NaN) are emitted as `null`, matching
    /// [`to_str`](Value::to_str).
    pub fn prettify(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out
    }

    /// Recursive worker for [`prettify`](Value::prettify).
    ///
    /// Writes this value into `out` at the given indent `level` (0 = top level).
    /// Under the calling convention, the *caller* emits the leading indent for
    /// this value's first line; this method emits only the value's content. For
    /// containers, that means the opening bracket, then each child on its own
    /// line (indented at `level + 1`), then the closing bracket indented at
    /// `level`. Leaves (`Null`/`Bool`/`Number`/`Str`) emit just their literal
    /// text.
    fn write_pretty(&self, out: &mut String, level: usize) {
        let indent = "  ".repeat(level);

        match self {
            Value::Array(items) => {
                out.push('[');
                out.push('\n');
                for (i, item) in items.iter().enumerate() {
                    out.push_str(&"  ".repeat(level + 1));
                    item.write_pretty(out, level + 1);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&indent);
                out.push(']');
            }
            Value::Object(items) => {
                out.push('{');
                out.push('\n');
                for (i, (key, val)) in items.iter().enumerate() {
                    out.push_str(&"  ".repeat(level + 1));
                    out.push('"');
                    out.push_str(key);
                    out.push_str("\": ");
                    val.write_pretty(out, level + 1);

                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(level));
                out.push('}');
            }
            Value::Bool(b) => out.push_str(&b.to_string()),
            Value::Null => out.push_str("null"),
            Value::Str(s) => {
                out.push('"');
                out.push_str(s);
                out.push('"');
            }
            Value::Number(n) => match n {
                JsonNumber::UInt(u) => out.push_str(&u.to_string()),
                JsonNumber::Int(i) => out.push_str(&i.to_string()),
                JsonNumber::Float(f) => {
                    if f.is_finite() {
                        out.push_str(&f.to_string())
                    } else {
                        out.push_str("null")
                    }
                }
            },
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_str())
    }
}
