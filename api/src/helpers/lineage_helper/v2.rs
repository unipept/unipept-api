use super::{create_lineages, get_name};
use serde::Serialize;
pub use paste::paste;
use datastore::{LineageStore, TaxonStore};

create_lineages!(
    LineageV2 {
        superkingdom,
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
    },
    lineages_v2
);
