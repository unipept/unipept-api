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
