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

  create_table "dataset_items", id: :integer, default: nil, force: :cascade do |t|
    t.bigint "dataset_id"
    t.string "name", limit: 160
    t.text "data", null: false
    t.integer "order"
  end

  create_table "datasets", id: :integer, default: nil, force: :cascade do |t|
    t.string "environment", limit: 160
    t.string "reference", limit: 500
    t.string "url", limit: 200
    t.string "project_website", limit: 200
  end

  create_table "ec_cross_references", id: :bigint, default: nil, force: :cascade do |t|
    t.bigint "uniprot_entry_id", null: false
    t.string "ec_number_code", limit: 15, null: false
    t.index ["uniprot_entry_id"], name: "idx_ec_cross_references_uniprot_entry_id"
  end

  create_table "ec_numbers", id: :integer, default: nil, force: :cascade do |t|
    t.string "code", limit: 15, null: false
    t.string "name", limit: 155, null: false
    t.index ["code"], name: "idx_ec_numbers_code"
  end

  create_table "go_cross_references", id: :bigint, default: nil, force: :cascade do |t|
    t.bigint "uniprot_entry_id", null: false
    t.string "go_term_code", limit: 15, null: false
    t.index ["uniprot_entry_id"], name: "idx_go_cross_references_uniprot_entry_id"
  end

  create_table "go_terms", id: :integer, default: nil, force: :cascade do |t|
    t.string "code", limit: 15, null: false
    t.enum "namespace", null: false, enum_type: "go_namespace"
    t.string "name", limit: 200, null: false
    t.index ["code"], name: "idx_go_terms_code"
  end

  create_table "interpro_cross_references", id: :bigint, default: nil, force: :cascade do |t|
    t.bigint "uniprot_entry_id", null: false
    t.string "interpro_entry_code", limit: 9, null: false
    t.index ["uniprot_entry_id"], name: "idx_interpro_cross_references_uniprot_entry_id"
  end

  create_table "interpro_entries", id: :integer, default: nil, force: :cascade do |t|
    t.string "code", limit: 9, null: false
    t.string "category", limit: 32, null: false
    t.string "name", limit: 160, null: false
  end

  create_table "lineages", primary_key: "taxon_id", id: :integer, default: nil, force: :cascade do |t|
    t.integer "superkingdom"
    t.integer "kingdom"
    t.integer "subkingdom"
    t.integer "superphylum"
    t.integer "phylum"
    t.integer "subphylum"
    t.integer "superclass"
    t.integer "class"
    t.integer "subclass"
    t.integer "superorder"
    t.integer "order"
    t.integer "suborder"
    t.integer "infraorder"
    t.integer "superfamily"
    t.integer "family"
    t.integer "subfamily"
    t.integer "tribe"
    t.integer "subtribe"
    t.integer "genus"
    t.integer "subgenus"
    t.integer "species_group"
    t.integer "species_subgroup"
    t.integer "species"
    t.integer "subspecies"
    t.integer "strain"
    t.integer "varietas"
    t.integer "forma"
  end

  create_table "peptides", id: :bigint, default: nil, force: :cascade do |t|
    t.bigint "sequence_id", null: false
    t.bigint "original_sequence_id", null: false
    t.bigint "uniprot_entry_id", null: false
    t.index ["original_sequence_id"], name: "idx_peptides_original_sequence_id"
    t.index ["sequence_id"], name: "idx_peptides_sequence_id"
    t.index ["uniprot_entry_id"], name: "idx_peptides_uniprot_entry_id"
  end

  create_table "sequences", id: :bigint, default: nil, force: :cascade do |t|
    t.string "sequence", limit: 50, null: false
    t.integer "lca"
    t.integer "lca_il"
    t.binary "fa"
    t.binary "fa_il"
    t.index ["lca"], name: "idx_sequences_lca"
    t.index ["lca_il"], name: "idx_sequences_lca_il"
    t.index ["sequence"], name: "idx_sequences_sequence"
  end

  create_table "taxons", id: :integer, default: nil, force: :cascade do |t|
    t.string "name", limit: 120, null: false
    t.enum "rank", enum_type: "rank_type"
    t.integer "parent_id"
    t.integer "valid_taxon", limit: 2, default: 1, null: false
  end

  create_table "uniprot_entries", id: :integer, default: nil, force: :cascade do |t|
    t.string "uniprot_accession_number", limit: 10, null: false
    t.integer "version", null: false
    t.integer "taxon_id", null: false
    t.enum "type", null: false, enum_type: "db_type"
    t.string "name", limit: 150, null: false
    t.text "protein", null: false
    t.index ["taxon_id"], name: "idx_uniprot_entries_taxon_id"
    t.index ["uniprot_accession_number"], name: "idx_uniprot_entries_uniprot_accession_number"
  end

  create_table "users", id: :integer, force: :cascade do |t|
    t.string "username", limit: 8, null: false
    t.integer "admin", limit: 1, default: 0, null: false
  end

  add_foreign_key "dataset_items", "datasets", name: "fk_dataset_items_datasets"
end
