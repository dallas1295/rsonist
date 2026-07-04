//! A small, zero-dependency JSON library.
//!
//! rsonist lets you parse JSON text into a [`Value`], navigate it, extract
//! typed primitives, and serialize a [`Value`] back to JSON text — without
//! pulling in `serde` or any external crate.
//!
//! # Parsing
//!
//! [`to_value`] turns JSON text into a [`Value`]. Drill in with
//! [`Value::get`] / [`Value::get_from`], then pull out a primitive with an
//! `as_*` method:
//!
//! ```
//! use rsonist::to_value;
//!
//! let json = r#"{"name": "alice", "scores": [10, 20, 30]}"#;
//! let root = to_value(json)?;
//!
//! let name = root.get("name").unwrap().as_str().unwrap();
//! let first_score = root.get("scores").unwrap().get_from(0).unwrap().as_i64().unwrap();
//!
//! assert_eq!(name, "alice");
//! assert_eq!(first_score, 10);
//! # Ok::<(), rsonist::ScanError>(())
//! ```
//!
//! # Building
//!
//! Construct a [`Value`] by hand, with [`JSONBuilder`], or via the `From`
//! impls on common Rust types, then serialize it with [`Value::to_str`].
//!
//! ```
//! use rsonist::JSONBuilder;
//!
//! let mut builder = JSONBuilder::new();
//! builder.add_pair("name", "alice".to_string());
//! let json = builder.build_val().to_str();
//! assert_eq!(json, r#"{"name": "alice"}"#);
//! ```

mod builder;
mod parse;
mod value;

pub use builder::JSONBuilder;
pub use parse::{ScanError, to_value};
pub use value::{JsonNumber, Value};
