# This file is auto-generated from the current state of the database. Instead
# of editing this file, please use the migrations feature of Active Record to
# incrementally modify your database, and then regenerate this schema definition.
#
# This file is the source Rails uses to define your schema when running `bin/rails
# db:schema:load`. When creating a new database, `bin/rails db:schema:load` tends to
# be faster and is potentially less error prone than running all of your
# migrations from scratch. Old migrations may fail to apply correctly if those
# migrations use external dependencies or application code.
#
# It's strongly recommended that you check this file into your version control system.

ActiveRecord::Schema[7.0].define(version: 0) do
  # These are extensions that must be enabled in order to support this database
  enable_extension "plpgsql"

  # Custom types defined in this database.
  # Note that some types may not work with other database engines. Be careful if changing database.
  create_enum "db_type", ["swissprot", "trembl"]
  create_enum "go_namespace", ["biological process", "molecular function", "cellular component"]
  create_enum "rank_type", ["no rank", "superkingdom", "kingdom", "subkingdom", "superphylum", "phylum", "subphylum", "superclass", "class", "subclass", "superorder", "order", "suborder", "infraorder", "superfamily", "family", "subfamily", "tribe", "subtribe", "genus", "subgenus", "species group", "species subgroup", "species", "subspecies", "strain", "varietas", "forma"]

  create_table "dataset_items", id: :integer, unsigned: true, force: :cascade do |t|
    t.bigint "dataset_id", unsigned: true
    t.string "name", limit: 160
    t.text "data", null: false
    t.integer "order"
  end

  create_table "datasets", id: :integer, unsigned: true, force: :cascade do |t|
    t.string "environment", limit: 160
    t.string "reference", limit: 500
    t.string "url", limit: 200
    t.string "project_website", limit: 200
  end

  create_table "ec_cross_references", id: :bigint, unsigned: true, force: :cascade do |t|
    t.bigint "uniprot_entry_id", null: false, unsigned: true
    t.string "ec_number_code", limit: 15, null: false
    t.index ["uniprot_entry_id"], name: "idx_ec_cross_references_uniprot_entry_id"
  end

  create_table "ec_numbers", id: :integer, unsigned: true, force: :cascade do |t|
    t.string "code", limit: 15, null: false
    t.string "name", limit: 155, null: false
    t.index ["code"], name: "idx_ec_numbers_code"
  end

  create_table "go_cross_references", id: :bigint, unsigned: true, force: :cascade do |t|
    t.bigint "uniprot_entry_id", null: false, unsigned: true
    t.string "go_term_code", limit: 15, null: false
    t.index ["uniprot_entry_id"], name: "idx_go_cross_references_uniprot_entry_id"
  end

  create_table "go_terms", id: :integer, unsigned: true, force: :cascade do |t|
    t.string "code", limit: 15, null: false
    t.enum "namespace", null: false, enum_type: "go_namespace"
    t.string "name", limit: 200, null: false
    t.index ["code"], name: "idx_go_terms_code"
  end

  create_table "interpro_cross_references", id: :bigint, unsigned: true, force: :cascade do |t|
    t.bigint "uniprot_entry_id", null: false, unsigned: true
    t.string "interpro_entry_code", limit: 9, null: false
    t.index ["uniprot_entry_id"], name: "idx_interpro_cross_references_uniprot_entry_id"
  end

  create_table "interpro_entries", id: :integer, unsigned: true, force: :cascade do |t|
    t.string "code", limit: 9, null: false
    t.string "category", limit: 32, null: false
    t.string "name", limit: 160, null: false
  end

  create_table "lineages", primary_key: "taxon_id", id: :integer, unsigned: true, force: :cascade do |t|
    t.integer "superkingdom", limit: 3
    t.integer "kingdom", limit: 3
    t.integer "subkingdom", limit: 3
    t.integer "superphylum", limit: 3
    t.integer "phylum", limit: 3
    t.integer "subphylum", limit: 3
    t.integer "superclass", limit: 3
    t.integer "class", limit: 3
    t.integer "subclass", limit: 3
    t.integer "superorder", limit: 3
    t.integer "order", limit: 3
    t.integer "suborder", limit: 3
    t.integer "infraorder", limit: 3
    t.integer "superfamily", limit: 3
    t.integer "family", limit: 3
    t.integer "subfamily", limit: 3
    t.integer "tribe", limit: 3
    t.integer "subtribe", limit: 3
    t.integer "genus", limit: 3
    t.integer "subgenus", limit: 3
    t.integer "species_group", limit: 3
    t.integer "species_subgroup", limit: 3
    t.integer "species", limit: 3
    t.integer "subspecies", limit: 3
    t.integer "strain", limit: 3
    t.integer "varietas", limit: 3
    t.integer "forma", limit: 3
  end

  create_table "peptides", id: :bigint, unsigned: true, force: :cascade do |t|
    t.bigint "sequence_id", null: false, unsigned: true
    t.bigint "original_sequence_id", null: false, unsigned: true
    t.bigint "uniprot_entry_id", null: false, unsigned: true
    t.index ["original_sequence_id"], name: "idx_peptides_original_sequence_id"
    t.index ["sequence_id"], name: "idx_peptides_sequence_id"
    t.index ["uniprot_entry_id"], name: "idx_peptides_uniprot_entry_id"
  end

  create_table "posts", id: :integer, unsigned: true, force: :cascade do |t|
    t.string "title", limit: 100, null: false
    t.text "content", null: false
    t.date "date", null: false
  end

  create_table "proteome_caches", primary_key: "proteome_id", id: :integer, limit: 3, unsigned: true, default: nil, force: :cascade do |t|
    t.text "json_sequences", limit: 16777215, null: false
  end

  create_table "proteome_cross_references", id: :integer, unsigned: true, force: :cascade do |t|
    t.integer "uniprot_entry_id", null: false, unsigned: true
    t.integer "proteome_id", limit: 3, null: false, unsigned: true
  end

  create_table "proteomes", id: :integer, limit: 3, unsigned: true, default: nil, force: :cascade do |t|
    t.string "proteome_accession_number", limit: 12, null: false
    t.string "proteome_name", limit: 145, null: false
    t.integer "taxon_id", limit: 3, unsigned: true
    t.binary "type_strain", limit: 1, default: 0b0, null: false
    t.binary "reference_proteome", limit: 1, default: 0b0, null: false
    t.string "strain", limit: 120
    t.string "assembly", limit: 45
    t.string "name", limit: 225
  end

  create_table "refseq_cross_references", id: :integer, unsigned: true, force: :cascade do |t|
    t.integer "uniprot_entry_id", null: false, unsigned: true
    t.string "protein_id", limit: 25
    t.string "sequence_id", limit: 25
  end

  create_table "sequences", id: :bigint, unsigned: true, force: :cascade do |t|
    t.string "sequence", limit: 50, null: false
    t.integer "lca", unsigned: true
    t.integer "lca_il", unsigned: true
    t.binary "fa", limit: 16777215
    t.binary "fa_il", limit: 16777215
    t.index ["lca"], name: "idx_sequences_lca"
    t.index ["lca_il"], name: "idx_sequences_lca_il"
    t.index ["sequence"], name: "idx_sequences_sequence"
  end

  create_table "taxons", id: :integer, unsigned: true, force: :cascade do |t|
    t.string "name", limit: 120, null: false
    t.enum "rank", enum_type: "rank_type"
    t.integer "parent_id", limit: 3, unsigned: true
    t.integer "valid_taxon", limit: 2, default: 1, null: false
  end

  create_table "uniprot_entries", id: :integer, unsigned: true, force: :cascade do |t|
    t.string "uniprot_accession_number", limit: 10, null: false
    t.integer "version", limit: 2, null: false, unsigned: true
    t.integer "taxon_id", limit: 3, null: false, unsigned: true
    t.enum "type", null: false, enum_type: "db_type"
    t.string "name", limit: 150, null: false
    t.text "protein", null: false
    t.index ["taxon_id"], name: "idx_uniprot_entries_taxon_id"
    t.index ["uniprot_accession_number"], name: "idx_uniprot_entries_uniprot_accession_number"
  end

  create_table "users", id: :integer, unsigned: true, force: :cascade do |t|
    t.string "username", limit: 8, null: false
    t.integer "admin", limit: 1, default: 0, null: false
  end

  add_foreign_key "dataset_items", "datasets", name: "fk_dataset_items_datasets"
end
