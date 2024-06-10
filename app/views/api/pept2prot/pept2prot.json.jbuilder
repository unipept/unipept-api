json.array! @input_order do |peptide|
  json.peptide peptide
  json.uniprot_id @result[peptide][:uniprot_id]
  json.protein_name @result[peptide][:name]
  json.taxon_id @result[peptide][:taxon_id]
  json.protein @result[peptide][:protein]
  if @extra_info
    json.taxon_name @result[peptide][:taxon][:name]
    json.ec_references @result[peptide][:ec_cross_references]
    json.go_references @result[peptide][:go_cross_references]
    json.interpro_references @result[peptide][:interpro_cross_references]
  end
end
