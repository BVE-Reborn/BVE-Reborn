use serde::{de, de::Visitor, Deserialize, Deserializer};
use std::{fmt, fmt::Formatter};

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Default)]
pub struct LooseNumericBool(pub bool);

impl<'de> Deserialize<'de> for LooseNumericBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LooseNumericBoolVisitor)
    }
}

struct LooseNumericBoolVisitor;

impl<'de> Visitor<'de> for LooseNumericBoolVisitor {
    type Value = LooseNumericBool;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(formatter, "Expecting integer to convert to bool.")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let parsed = parse_loose_numeric_bool(v);

        parsed.ok_or_else(|| serde::de::Error::custom(format!("Error parsing the numeric bool {}", v)))
    }
}

pub fn parse_loose_numeric_bool(input: &str) -> Option<LooseNumericBool> {
    let mut filtered: String = input.chars().filter(|c| !c.is_whitespace()).collect();

    let first_letter = filtered.chars().next().as_ref().map(char::to_ascii_lowercase);
    match first_letter {
        Some('t') => return Some(LooseNumericBool(true)),
        Some('f') => return Some(LooseNumericBool(false)),
        _ => {}
    }

    while !filtered.is_empty() {
        let parsed: Result<i64, _> = filtered.parse();
        match parsed {
            Ok(v) => {
                let output = v != 0;
                return Some(LooseNumericBool(output));
            }
            Err(_) => filtered.pop(),
        };
    }

    None
}

#[cfg(test)]
mod test {
    use crate::parse::util::{parse_loose_numeric_bool, LooseNumericBool};
    use serde_test::{assert_de_tokens, Token};

    #[bve_derive::bve_test]
    #[test]
    fn loose_bool() {
        let b = LooseNumericBool(false);
        assert_de_tokens(&b, &[Token::Str("0")]);
        assert_de_tokens(&b, &[Token::Str("0xxxx0")]);
        assert_de_tokens(&b, &[Token::Str("0.1")]);
        let b = LooseNumericBool(true);
        assert_de_tokens(&b, &[Token::Str("1")]);
        assert_de_tokens(&b, &[Token::Str("1xxxx0")]);
        assert_de_tokens(&b, &[Token::Str("1.1")]);
        assert_eq!(parse_loose_numeric_bool(""), None);
        assert_eq!(parse_loose_numeric_bool("True"), Some(LooseNumericBool(true)));
        assert_eq!(parse_loose_numeric_bool("False"), Some(LooseNumericBool(false)));
        assert_eq!(parse_loose_numeric_bool("t"), Some(LooseNumericBool(true)));
        assert_eq!(parse_loose_numeric_bool("f"), Some(LooseNumericBool(false)));
    }
}
