use serde::Deserialize;

use crate::controllers::request::Flag;

pub mod pept2ec;
pub mod pept2funct;
pub mod pept2go;
pub mod pept2interpro;
pub mod pept2lca;
pub mod pept2prot;
pub mod pept2taxa;
pub mod peptinfo;
pub mod protinfo;
pub mod taxa2lca;
pub mod taxa2tree;
pub mod taxonomy;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Either<T, U> {
    Left(T),
    Right(U)
}

impl From<&Either<u32, String>> for u32 {
    fn from(either: &Either<u32, String>) -> Self {
        match either {
            Either::Left(n) => *n,
            Either::Right(s) => s.parse().unwrap_or_default()
        }
    }
}

pub fn default_equate_il() -> Flag {
    Flag(true)
}

pub fn default_extra() -> Flag {
    Flag(false)
}

pub fn default_tryptic() -> Flag {
    Flag(false)
}

pub fn default_domains() -> Flag {
    Flag(false)
}

pub fn default_names() -> Flag {
    Flag(false)
}

pub fn default_descendants() -> Flag {
    Flag(false)
}

pub fn default_descendants_ranks() -> Vec<String> {
    vec![String::from("species")]
}

pub fn default_link() -> Flag {
    Flag(false)
}

pub fn default_compact() -> Flag {
    Flag(false)
}

pub fn default_cutoff() -> usize {
    10000
}

pub fn default_validate_taxa() -> Flag {
    Flag(true)
}
