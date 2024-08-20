use datastore::{LineageStore, TaxonStore};
pub use paste::paste;
use serde::Serialize;

use super::create_lineages;

const RANKS_V1: [&str; 28] = [
    "superkingdom",
    "kingdom",
    "subkingdom",
    "superphylum",
    "phylum",
    "subphylum",
    "superclass",
    "class",
    "subclass",
    "infraclass",
    "superorder",
    "order",
    "suborder",
    "infraorder",
    "parvorder",
    "superfamily",
    "family",
    "subfamily",
    "tribe",
    "subtribe",
    "genus",
    "subgenus",
    "species_group",
    "species_subgroup",
    "species",
    "subspecies",
    "varietas",
    "forma"
];

create_lineages!(
    superkingdom,
    kingdom,
    subkingdom,
    superphylum,
    phylum,
    subphylum,
    superclass,
    class,
    subclass,
    infraclass,
    superorder,
    order,
    suborder,
    infraorder,
    parvorder,
    superfamily,
    family,
    subfamily,
    tribe,
    subtribe,
    genus,
    subgenus,
    species_group,
    species_subgroup,
    species,
    subspecies,
    varietas,
    forma
);
