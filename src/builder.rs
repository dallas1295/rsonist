use std::collections::BTreeMap;

use crate::value::Value;

/// A builder for constructing JSON objects incrementally.
///
/// Useful when you're assembling an object field by field and want it to read
/// top-to-bottom rather than nesting `Value` constructors by hand. When done,
/// call [`build`](JSONBuilder::build) to get back a [`Value`].
///
/// # Example
///
/// ```
/// use rsonist::JSONBuilder;
///
/// let mut builder = JSONBuilder::new();
/// builder.add_pair("name".to_string(), "alice".to_string());
/// builder.add_pair("age".to_string(), 30);
///
/// let value = builder.build();
/// assert_eq!(value.get("name").unwrap().as_str().unwrap(), "alice");
/// assert_eq!(value.get("age").unwrap().as_i64().unwrap(), 30);
/// ```
pub struct JSONBuilder {
    entries: BTreeMap<String, Value>,
}

impl JSONBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        JSONBuilder {
            entries: BTreeMap::new(),
        }
    }

    /// Inserts a key/value pair. Any type that implements [`Into<Value>`]
    /// (the integer/float/bool/String/Vec/HashMap types) can be passed
    /// directly as the value.
    pub fn add_pair(&mut self, key: &str, value: impl Into<Value>) {
        self.entries.insert(key.to_string(), value.into());
    }

    pub fn add_object<F>(&mut self, key: &str, f: F)
    where
        F: FnOnce(&mut Value),
    {
        let mut v = Value::Object(BTreeMap::new());
        f(&mut v);
        self.entries.insert(key.to_string(), v);
    }

    pub fn add_array<F>(&mut self, key: &str, f: F)
    where
        F: FnOnce(&mut Vec<Value>),
    {
        let mut arr = Vec::new();
        f(&mut arr);
        self.entries.insert(key.to_string(), Value::Array(arr));
    }

    /// Consumes the builder and returns the assembled object as a [`Value`].
    pub fn build(self) -> Value {
        Value::Object(self.entries)
    }
}

impl Default for JSONBuilder {
    fn default() -> Self {
        Self::new()
    }
}
