use super::{create_lineages, get_name};
use serde::Serialize;
pub use paste::paste;
use datastore::{LineageStore, TaxonStore};

create_lineages!(
    LineageV1 {
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
    },
    lineages_v1
);
