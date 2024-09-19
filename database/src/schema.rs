use diesel::table;

table! {
    uniprot_entries (id) {
        id -> Unsigned<Integer>,
        uniprot_accession_number -> Char,
        version -> Unsigned<Integer>,
        taxon_id -> Unsigned<Integer>,
        #[sql_name = "type"]
        db_type -> Char,
        name -> Varchar,
        protein -> Text,
        fa -> Text
    }
}
