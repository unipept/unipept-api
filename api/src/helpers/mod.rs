pub mod ec_helper;
pub mod fa_helper;
pub mod go_helper;
pub mod interpro_helper;
pub mod lca_helper;
pub mod lineage_helper;
pub mod tree_helper;

fn is_zero(num: &u32) -> bool {
    *num == 0
}

pub fn sanitize_peptides(peptides: Vec<String>) -> Vec<String> {
    peptides
        .into_iter()
        .map(|s| s.trim_end().to_uppercase())
        .collect()
}

pub fn sanitize_proteins(proteins: Vec<String>) -> Vec<String> {
    proteins
        .into_iter()
        .map(|s| s.trim_end().to_string())
        .collect()
}
