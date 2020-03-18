use crate::parse::kvp::{parse_kvp_file, FromKVPFile, KVPGenericWarning, KVPSymbols, DAT_LIKE};
use crate::parse::util::strip_comments;
use crate::parse::KVPFileParser;
pub use sections::*;
use std::fmt;

mod sections;

#[must_use]
pub fn parse_train_dat(input: &str) -> (ParsedTrainDat, Vec<KVPGenericWarning>) {
    let lower = strip_comments(input, ';').to_lowercase();
    let kvp_file = parse_kvp_file(&lower, DAT_LIKE);

    ParsedTrainDat::from_kvp_file(&kvp_file)
}

impl fmt::Display for ParsedTrainDat {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!()
    }
}

impl KVPFileParser for ParsedTrainDat {
    const SYMBOLS: KVPSymbols = DAT_LIKE;
    const COMMENT: char = ';';
}
