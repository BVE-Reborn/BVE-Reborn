use crate::parse::{
    kvp::{KVPSymbols, ANIMATED_LIKE},
    KVPFileParser,
};
pub use sections::*;

mod sections;

impl KVPFileParser for ParsedSoundCfg {
    const COMMENT: char = ';';
    const SYMBOLS: KVPSymbols = ANIMATED_LIKE;
}
