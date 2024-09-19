use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    LoadError(#[from] LoadIndexError)
}

#[derive(Error, Debug)]
pub enum LoadIndexError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Error while loading suffix array: {0}")]
    LoadSuffixArrayError(String),
    #[error("Error while loading proteins: {0}")]
    LoadProteinsErrors(String),
    #[error("Error while loading taxonomy: {0}")]
    LoadTaxonomyError(String)
}
