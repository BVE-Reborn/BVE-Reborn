//! Generic parser for key-value pair format. Not quite toml, not quite INI.
//!
//! No frills format. Does not deal with casing, comments, etc. That must be
//! dealt with ahead of time. This just deserializes the file as is. However
//! whitespace is trimmed off the edges of values
//!
//! There may be arbitrary duplicates.
//!
//! The first section before a section header is always the unnamed section `None`.
//! This differs from an empty section name `Some("")`
//!
//! ```ini
//! value1
//! key1 = some_value
//!
//! [section1]
//! value
//! key = value
//! key = value
//! ```

use crate::parse::Span;
pub use parse::*;
pub use traits::*;

mod parse;
#[cfg(test)]
mod tests;
mod traits;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KVPFile<'s> {
    pub sections: Vec<KVPSection<'s>>,
}

impl<'s> Default for KVPFile<'s> {
    fn default() -> Self {
        Self {
            sections: Vec::default(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KVPSection<'s> {
    pub name: Option<&'s str>,
    pub span: Span,
    pub fields: Vec<KVPField<'s>>,
}

impl<'s> Default for KVPSection<'s> {
    fn default() -> Self {
        Self {
            name: None,
            span: Span::from_line(0),
            fields: Vec::default(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KVPField<'s> {
    pub span: Span,
    pub data: ValueData<'s>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ValueData<'s> {
    KeyValuePair { key: &'s str, value: &'s str },
    Value { value: &'s str },
}
