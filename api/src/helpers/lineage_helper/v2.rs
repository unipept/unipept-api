use datastore::{LineageStore, TaxonStore};
pub use paste::paste;
use serde::Serialize;

use super::create_lineages;

create_lineages!(
    domain,
    realm,
    kingdom,
    subkingdom,
    superphylum,
    phylum,
    subphylum,
    superclass,
    class,
    subclass,
    superorder,
    order,
    suborder,
    infraorder,
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
    strain,
    varietas,
    forma
);
