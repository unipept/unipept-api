//! The annotation aggregation, which is the only per-hit work the API does itself.
//!
//! `calculate_fa` is pure — no index, no datastore and no OpenSearch — so it can be measured
//! directly rather than through a request.

use criterion::{black_box, Criterion};
use index::{fa_compression::algorithm1::encode, ProteinInfo};
use unipept_api::helpers::fa_helper::calculate_fa;

/// A result set the shape a real one has: many proteins drawing from a small pool of terms, so
/// most occurrences are of a term already counted.
fn corpus(protein_count: usize) -> (Vec<String>, Vec<Vec<u8>>) {
    let terms: Vec<String> = (0..40)
        .map(|i| match i % 3 {
            0 => format!("EC:1.{}.1.-", i),
            1 => format!("GO:{:07}", 9000 + i),
            _ => format!("IPR:IPR{:06}", 16000 + i)
        })
        .collect();

    let accessions: Vec<String> = (0..protein_count).map(|i| format!("P{:05}", i)).collect();
    let annotations: Vec<Vec<u8>> = (0..protein_count)
        .map(|i| {
            let picked: Vec<&str> = (0..5).map(|k| terms[(i * 7 + k * 11) % terms.len()].as_str()).collect();
            encode(&picked.join(";"))
        })
        .collect();

    (accessions, annotations)
}

pub fn fa_benchmark(c: &mut Criterion) {
    for protein_count in [1_000usize, 10_000] {
        let (accessions, annotations) = corpus(protein_count);
        let proteins: Vec<ProteinInfo> = (0..protein_count)
            .map(|i| ProteinInfo {
                taxon: 1,
                uniprot_accession: accessions[i].as_str(),
                annotations: annotations[i].as_slice()
            })
            .collect();

        c.bench_function(&format!("calculate_fa/{protein_count}"), |b| {
            b.iter(|| black_box(calculate_fa(black_box(&proteins))))
        });
    }
}
