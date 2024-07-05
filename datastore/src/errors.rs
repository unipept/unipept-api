use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataStoreError {
    #[error("Sample store error: {0}")]
    SampleStoreError(#[from] SampleStoreError),
    #[error("Ec store error: {0}")]
    EcStoreError(#[from] EcStoreError),
    #[error("Go store error: {0}")]
    GoStoreError(#[from] GoStoreError),
    #[error("Interpro store error: {0}")]
    InterproStoreError(#[from] InterproStoreError),
    #[error("Lineage store error: {0}")]
    LineageStoreError(#[from] LineageStoreError),
    #[error("Taxon store error: {0}")]
    TaxonStoreError(#[from] TaxonStoreError)
}

#[derive(Error, Debug)]
pub enum SampleStoreError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    SerdeError(#[from] serde_json::Error)
}

#[derive(Error, Debug)]
pub enum EcStoreError {
    #[error("{0}")]
    IoError(#[from] std::io::Error)
}

#[derive(Error, Debug)]
pub enum GoStoreError {
    #[error("{0}")]
    IoError(#[from] std::io::Error)
}

#[derive(Error, Debug)]
pub enum InterproStoreError {
    #[error("{0}")]
    IoError(#[from] std::io::Error)
}

#[derive(Error, Debug)]
pub enum LineageStoreError {
    #[error("{0}")]
    IoError(#[from] std::io::Error)
}

#[derive(Error, Debug)]
pub enum TaxonStoreError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("Lineage rank `{0}` not found in lineage store")]
    InvalidRankError(String)
}
