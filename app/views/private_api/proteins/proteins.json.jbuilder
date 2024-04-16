# json.lca @lca_taxon ? @lca_taxon.id : -1
# json.common_lineage(@common_lineage.map(&:id))
# json.proteins @entries do |entry|
#   json.uniprotAccessionId entry.uniprot_accession_number
#   json.name entry.name
#   json.organism entry.taxon_id
#   json.ecNumbers(entry.ec_cross_references.map(&:ec_number_code))
#   json.goTerms(entry.go_cross_references.map(&:go_term_code))
#   json.interproEntries(entry.interpro_cross_references.map(&:interpro_entry_code))
# end

json.lca @lca
json.common_lineage @common_lineage
json.proteins @proteins do |protein|
  json.uniprotAccessionId protein[:uniprot_accession_number]
  json.name protein[:name]
  json.organism protein[:organism]
  json.ecNumbers protein[:ec_numbers]
  json.goTerms protein[:go_terms]
  json.interproEntries protein[:interpro_entries]
end
