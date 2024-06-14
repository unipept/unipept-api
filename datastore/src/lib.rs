mod sample_store;
mod ec_store;
mod go_store;
mod interpro_store;

pub use sample_store::SampleStore;
use ec_store::EcStore;
use go_store::GoStore;
use interpro_store::InterproStore;

pub struct DataStore {
    version: String,
    sample_store: SampleStore,
    ec_store: EcStore,
    go_store: GoStore,
    interpro_store: InterproStore
}

impl DataStore {
    pub fn try_from_files(version: &str, sample_file: &str, ec_file: &str, go_file: &str, interpro_file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            version: version.to_string(),
            sample_store: SampleStore::try_from_file(sample_file)?,
            ec_store: EcStore::try_from_file(ec_file)?,
            go_store: GoStore::try_from_file(go_file)?,
            interpro_store: InterproStore::try_from_file(interpro_file)?
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
}
