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
/// builder.add_pair("name", "alice".to_string());
/// builder.add_pair("age", 30);
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
    /// (the integer/float/bool/String/Vec/BTreeMap types) can be passed
    /// directly as the value.
    pub fn add_pair(&mut self, key: &str, value: impl Into<Value>) -> &mut Self {
        self.entries.insert(key.to_string(), value.into());
        self
    }

    /// Inserts a nested object under the given key.
    ///
    /// The closure receives a fresh [`JSONBuilder`] for the child object.
    /// This is useful for building nested structures top-to-bottom:
    ///
    /// ```
    /// use rsonist::{JSONBuilder, Value};
    ///
    /// let mut builder = JSONBuilder::new();
    /// builder.add_pair("name", "alice".to_string());
    /// builder.add_object("meta", |c| {
    ///     c.add_pair("age", 30);
    ///     c.add_pair("active", true);
    /// });
    /// let value = builder.build();
    /// assert_eq!(value.get("meta").unwrap().get("age").unwrap().as_i64().unwrap(), 30);
    /// ```
    pub fn add_object<F>(&mut self, key: &str, f: F) -> &mut Self
    where
        F: FnOnce(&mut JSONBuilder),
    {
        let mut child = JSONBuilder::new();
        f(&mut child);
        self.entries.insert(key.to_string(), child.build());
        self
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
