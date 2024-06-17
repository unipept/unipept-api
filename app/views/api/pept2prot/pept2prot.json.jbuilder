@input_order.map do |peptide|
  json.array! @result[peptide].map do |prot|
    json.peptide peptide
    json.uniprot_id prot[:uniprot_id]
    json.protein_name prot[:name]
    json.taxon_id prot[:taxon_id]
    json.protein prot[:protein]
    if @extra_info
      json.taxon_name prot[:taxon][:name]
      json.ec_references prot[:ec_cross_references]
      json.go_references prot[:go_cross_references]
      json.interpro_references prot[:interpro_cross_references]
    end
  end
end
