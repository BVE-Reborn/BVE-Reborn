use crate::parse::{kvp::FromKVPValue, PrettyPrintResult};
use bve_derive::FromKVPSection;
use std::io;

#[derive(Debug, Default, Clone, PartialEq, FromKVPSection)]
pub struct VersionSection {
    #[kvp(bare)]
    pub version: Version,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Version {
    BVE120,
    BVE121,
    BVE122,
    BVE2,
    OpenBVE { version: String },
}

impl Default for Version {
    fn default() -> Self {
        Self::BVE2
    }
}

impl FromKVPValue for Version {
    fn from_kvp_value(value: &str) -> Option<Self> {
        Some(match value {
            "bve1200000" => Self::BVE120,
            "bve1210000" => Self::BVE121,
            "bve1220000" => Self::BVE122,
            "bve2000000" | "openbve" => Self::BVE2,
            _ if value.starts_with("openbve") => Self::OpenBVE {
                version: String::from(&value[7..]),
            },
            _ => return None,
        })
    }
}

impl PrettyPrintResult for Version {
    fn fmt(&self, _indent: usize, out: &mut dyn io::Write) -> io::Result<()> {
        write!(out, "{}", match self {
            Self::BVE120 => "BVE1200000",
            Self::BVE121 => "BVE1210000",
            Self::BVE122 => "BVE1220000",
            Self::BVE2 => "BVE2000000",
            Self::OpenBVE { version } => version.as_str(),
        })?;
        Ok(())
    }
}
