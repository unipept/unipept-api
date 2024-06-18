use datastore::{LineageStore, TaxonStore};
use serde::Serialize;

use super::get_name;

// TODO: write macros to generate these structs

#[derive(Serialize)]
#[serde(untagged)]
pub enum LineageV1 {
    Lineage {
        superkingdom_id: Option<i32>,
        kingdom_id: Option<i32>,
        subkingdom_id: Option<i32>,
        superphylum_id: Option<i32>,
        phylum_id: Option<i32>,
        subphylum_id: Option<i32>,
        superclass_id: Option<i32>,
        class_id: Option<i32>,
        subclass_id: Option<i32>,
        infraclass_id: Option<i32>,
        superorder_id: Option<i32>,
        order_id: Option<i32>,
        suborder_id: Option<i32>,
        infraorder_id: Option<i32>,
        parvorder_id: Option<i32>,
        superfamily_id: Option<i32>,
        family_id: Option<i32>,
        subfamily_id: Option<i32>,
        tribe_id: Option<i32>,
        subtribe_id: Option<i32>,
        genus_id: Option<i32>,
        subgenus_id: Option<i32>,
        species_group_id: Option<i32>,
        species_subgroup_id: Option<i32>,
        species_id: Option<i32>,
        subspecies_id: Option<i32>,
        varietas_id: Option<i32>,
        forma_id: Option<i32>
    },
    LineageWithNames {
        superkingdom_id: Option<i32>,
        superkingdom_name: Option<String>,
        kingdom_id: Option<i32>,
        kingdom_name: Option<String>,
        subkingdom_id: Option<i32>,
        subkingdom_name: Option<String>,
        superphylum_id: Option<i32>,
        superphylum_name: Option<String>,
        phylum_id: Option<i32>,
        phylum_name: Option<String>,
        subphylum_id: Option<i32>,
        subphylum_name: Option<String>,
        superclass_id: Option<i32>,
        superclass_name: Option<String>,
        class_id: Option<i32>,
        class_name: Option<String>,
        subclass_id: Option<i32>,
        subclass_name: Option<String>,
        infraclass_id: Option<i32>,
        infraclass_name: Option<String>,
        superorder_id: Option<i32>,
        superorder_name: Option<String>,
        order_id: Option<i32>,
        order_name: Option<String>,
        suborder_id: Option<i32>,
        suborder_name: Option<String>,
        infraorder_id: Option<i32>,
        infraorder_name: Option<String>,
        parvorder_id: Option<i32>,
        parvorder_name: Option<String>,
        superfamily_id: Option<i32>,
        superfamily_name: Option<String>,
        family_id: Option<i32>,
        family_name: Option<String>,
        subfamily_id: Option<i32>,
        subfamily_name: Option<String>,
        tribe_id: Option<i32>,
        tribe_name: Option<String>,
        subtribe_id: Option<i32>,
        subtribe_name: Option<String>,
        genus_id: Option<i32>,
        genus_name: Option<String>,
        subgenus_id: Option<i32>,
        subgenus_name: Option<String>,
        species_group_id: Option<i32>,
        species_group_name: Option<String>,
        species_subgroup_id: Option<i32>,
        species_subgroup_name: Option<String>,
        species_id: Option<i32>,
        species_name: Option<String>,
        subspecies_id: Option<i32>,
        subspecies_name: Option<String>,
        varietas_id: Option<i32>,
        varietas_name: Option<String>,
        forma_id: Option<i32>,
        forma_name: Option<String>
    }
}

pub fn lineages_v1(taxon_id: u32, names: bool, lineage_store: &LineageStore, taxon_store: &TaxonStore) -> Option<LineageV1> {
    let lineage = lineage_store.get(taxon_id)?;

    if names {
        Some(LineageV1::LineageWithNames {
            superkingdom_id: lineage.superkingdom,
            superkingdom_name: get_name(lineage.superkingdom, taxon_store),
            kingdom_id: lineage.kingdom,
            kingdom_name: get_name(lineage.kingdom, taxon_store),
            subkingdom_id: lineage.subkingdom,
            subkingdom_name: get_name(lineage.subkingdom, taxon_store),
            superphylum_id: lineage.superphylum,
            superphylum_name: get_name(lineage.superphylum, taxon_store),
            phylum_id: lineage.phylum,
            phylum_name: get_name(lineage.phylum, taxon_store),
            subphylum_id: lineage.subphylum,
            subphylum_name: get_name(lineage.subphylum, taxon_store),
            superclass_id: lineage.superclass,
            superclass_name: get_name(lineage.superclass, taxon_store),
            class_id: lineage.class,
            class_name: get_name(lineage.class, taxon_store),
            subclass_id: lineage.subclass,
            subclass_name: get_name(lineage.subclass, taxon_store),
            infraclass_id: lineage.infraclass,
            infraclass_name: get_name(lineage.infraclass, taxon_store),
            superorder_id: lineage.superorder,
            superorder_name: get_name(lineage.superorder, taxon_store),
            order_id: lineage.order,
            order_name: get_name(lineage.order, taxon_store),
            suborder_id: lineage.suborder,
            suborder_name: get_name(lineage.suborder, taxon_store),
            infraorder_id: lineage.infraorder,
            infraorder_name: get_name(lineage.infraorder, taxon_store),
            parvorder_id: lineage.parvorder,
            parvorder_name: get_name(lineage.parvorder, taxon_store),
            superfamily_id: lineage.superfamily,
            superfamily_name: get_name(lineage.superfamily, taxon_store),
            family_id: lineage.family,
            family_name: get_name(lineage.family, taxon_store),
            subfamily_id: lineage.subfamily,
            subfamily_name: get_name(lineage.subfamily, taxon_store),
            tribe_id: lineage.tribe,
            tribe_name: get_name(lineage.tribe, taxon_store),
            subtribe_id: lineage.subtribe,
            subtribe_name: get_name(lineage.subtribe, taxon_store),
            genus_id: lineage.genus,
            genus_name: get_name(lineage.genus, taxon_store),
            subgenus_id: lineage.subgenus,
            subgenus_name: get_name(lineage.subgenus, taxon_store),
            species_group_id: lineage.species_group,
            species_group_name: get_name(lineage.species_group, taxon_store),
            species_subgroup_id: lineage.species_subgroup,
            species_subgroup_name: get_name(lineage.species_subgroup, taxon_store),
            species_id: lineage.species,
            species_name: get_name(lineage.species, taxon_store),
            subspecies_id: lineage.subspecies,
            subspecies_name: get_name(lineage.subspecies, taxon_store),
            varietas_id: lineage.varietas,
            varietas_name: get_name(lineage.varietas, taxon_store),
            forma_id: lineage.forma,
            forma_name: get_name(lineage.forma, taxon_store)
        })
    } else {
        Some(LineageV1::Lineage {
            superkingdom_id: lineage.superkingdom,
            kingdom_id: lineage.kingdom,
            subkingdom_id: lineage.subkingdom,
            superphylum_id: lineage.superphylum,
            phylum_id: lineage.phylum,
            subphylum_id: lineage.subphylum,
            superclass_id: lineage.superclass,
            class_id: lineage.class,
            subclass_id: lineage.subclass,
            infraclass_id: lineage.infraclass,
            superorder_id: lineage.superorder,
            order_id: lineage.order,
            suborder_id: lineage.suborder,
            infraorder_id: lineage.infraorder,
            parvorder_id: lineage.parvorder,
            superfamily_id: lineage.superfamily,
            family_id: lineage.family,
            subfamily_id: lineage.subfamily,
            tribe_id: lineage.tribe,
            subtribe_id: lineage.subtribe,
            genus_id: lineage.genus,
            subgenus_id: lineage.subgenus,
            species_group_id: lineage.species_group,
            species_subgroup_id: lineage.species_subgroup,
            species_id: lineage.species,
            subspecies_id: lineage.subspecies,
            varietas_id: lineage.varietas,
            forma_id: lineage.forma
        })
    }
}
