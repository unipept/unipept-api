mod ec_store;
mod errors;
mod go_store;
mod interpro_store;
mod lineage_store;
mod sample_store;
mod taxon_store;
mod reference_proteome_store;

pub use ec_store::EcStore;
pub use errors::DataStoreError;
pub use go_store::GoStore;
pub use interpro_store::InterproStore;
pub use lineage_store::{Lineage, LineageStore};
pub use sample_store::SampleStore;
pub use taxon_store::{LineageRank, TaxonStore};
pub use reference_proteome_store::ReferenceProteomeStore;

pub struct DataStore {
    version: String,
    sample_store: SampleStore,
    ec_store: EcStore,
    go_store: GoStore,
    interpro_store: InterproStore,
    reference_proteome_store: ReferenceProteomeStore,
    lineage_store: LineageStore,
    taxon_store: TaxonStore
}

impl DataStore {
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_files(
        version_file: &str,
        sample_file: &str,
        ec_file: &str,
        go_file: &str,
        interpro_file: &str,
        reference_proteome_file: &str,
        lineage_file: &str,
        taxon_file: &str
    ) -> Result<Self, DataStoreError> {
        let version = std::fs::read_to_string(version_file).map_err(
            |_err| DataStoreError::FileNotFound(version_file.to_string())
        )?;
        Ok(Self {
            version: version.trim_end().to_string(),
            sample_store: SampleStore::try_from_file(sample_file)?,
            ec_store: EcStore::try_from_file(ec_file)?,
            go_store: GoStore::try_from_file(go_file)?,
            interpro_store: InterproStore::try_from_file(interpro_file)?,
            reference_proteome_store: ReferenceProteomeStore::try_from_file(reference_proteome_file)?,
            lineage_store: LineageStore::try_from_file(lineage_file)?,
            taxon_store: TaxonStore::try_from_file(taxon_file)?
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn sample_store(&self) -> &SampleStore {
        &self.sample_store
    }

    pub fn ec_store(&self) -> &EcStore {
        &self.ec_store
    }

    pub fn go_store(&self) -> &GoStore {
        &self.go_store
    }

    pub fn interpro_store(&self) -> &InterproStore {
        &self.interpro_store
    }
    
    pub fn reference_proteome_store(&self) -> &ReferenceProteomeStore { &self.reference_proteome_store }

    pub fn lineage_store(&self) -> &LineageStore {
        &self.lineage_store
    }

    pub fn taxon_store(&self) -> &TaxonStore {
        &self.taxon_store
    }
}
