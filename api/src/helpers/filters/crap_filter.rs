use std::collections::HashSet;
use index::ProteinInfo;
use crate::helpers::filters::protein_filter::ProteinFilter;
use crate::helpers::filters::UniprotFilter;

pub struct CrapFilter {
    protein_filter: ProteinFilter,
}

impl UniprotFilter for CrapFilter {
    fn filter(&self, protein: &ProteinInfo) -> bool {
        self.protein_filter.filter(protein)
    }
}

impl Default for CrapFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl CrapFilter {
    pub fn new() -> Self {
        CrapFilter {
            protein_filter: ProteinFilter::new(Self::get_crap_proteins()),
        }
    }

    fn get_crap_proteins() -> HashSet<String> {
        let crap_accessions = vec![
            "P02769", "P0DTE7", "P0DTE8", "P0DUB6", "P02662", "P02663",
            "P02666", "P02668", "P00766", "P00767", "P13645", "O77727",
            "P35527", "Q15323", "Q14532", "O76011", "Q92764", "O76013",
            "O76014", "O76015", "O76009", "P01920", "P02534", "P02539",
            "P35908", "P04264", "P15241", "P25691", "P02444", "P81054",
            "P02445", "P02443", "P02441", "Q02958", "P02438", "P02439",
            "P02440", "P08131", "Q14533", "Q9NSB4", "P78385", "Q9NSB2",
            "P78386", "O43790", "P26372", "P00711", "Q7M135", "P00792",
            "P00791", "Q10735", "P30879", "P0C1U8", "P00760", "Q29463",
            "A0A8K0BFD9", "P02768", "P01008", "D6RCN3", "P61769", "P55957",
            "P00915", "P00918", "P04040", "P07339", "P08311", "P01031",
            "P02741", "P00167", "P99999", "P01133", "P05413", "P06396",
            "Q9BX51", "A0A2R8Y5E5", "P69905", "P68871", "P01344", "P10145",
            "P06732", "P00709", "P80384", "P61626", "P02144", "Q15843",
            "P15559", "U3KQG7", "P01127", "P62937", "A0A0A0MRQ5", "P01112",
            "P02753", "P00441", "B8ZZN6", "P12081", "P10636", "P10599",
            "P01375", "P02787", "P02788", "P51965", "O00762", "A8MUA9",
            "P62979", "P32503", "P00004", "P00921", "P00330", "P00883",
            "P00698", "P68082", "P01012", "P00722", "P00366", "A0A5J6CYK8",
            "A0A182BM84", "P15252"
        ];

        crap_accessions.into_iter().map(String::from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protein_in_crap_filter() {
        let filter = CrapFilter::new();
        let protein_in_filter = ProteinInfo {
            taxon: 1,
            uniprot_accession: "P68082".to_string(),
            functional_annotations: "GO:0001234;GO:0005678".to_string()
        };

        assert!(filter.filter(&protein_in_filter));
    }

    #[test]
    fn test_protein_not_in_crap_filter() {
        let filter = CrapFilter::new();
        let protein_in_filter = ProteinInfo {
            taxon: 1,
            uniprot_accession: "PXXXXX".to_string(),
            functional_annotations: "GO:0001234;GO:0005678".to_string()
        };

        assert!(!filter.filter(&protein_in_filter));
    }
}
