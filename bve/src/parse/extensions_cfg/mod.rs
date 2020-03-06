use crate::parse::kvp::{parse_kvp_file, FromKVPFile, KVPGenericWarning, ANIMATED_LIKE};
use crate::parse::util::strip_comments;
pub use sections::*;

mod sections;

#[must_use]
pub fn parse_extensions_cfg(input: &str) -> (ParsedExtensionsCfg, Vec<KVPGenericWarning>) {
    let lower = strip_comments(input, ';').to_lowercase();
    let kvp_file = parse_kvp_file(&lower, ANIMATED_LIKE);

    ParsedExtensionsCfg::from_kvp_file(&kvp_file)
}
