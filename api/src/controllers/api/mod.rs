use serde::Deserialize;

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

pub fn default_equate_il() -> bool {
    true
}

pub fn default_extra() -> bool {
    false
}

pub fn default_tryptic() -> bool {
    false
}

pub fn default_domains() -> bool {
    false
}

pub fn default_names() -> bool {
    false
}

pub fn default_descendants() -> bool { false }

pub fn default_descendants_ranks() -> Vec<String> {
    vec![String::from("species")]
}

pub fn default_link() -> bool {
    false
}
