use crate::controllers::request::Flag;

pub mod pept2data;

pub fn default_equate_il() -> Flag {
    Flag(true)
}

pub fn default_include_fa() -> Flag {
    Flag(false)
}

pub fn default_tryptic() -> Flag {
    Flag(false)
}

pub fn default_report_taxa() -> Flag {
    Flag(false)
}
