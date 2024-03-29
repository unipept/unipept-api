json.array! @input_order do |uniprot_id|
  if @result.key? uniprot_id
    json.protein uniprot_id

    json.ec @result[uniprot_id][:ec]
    json.go @result[uniprot_id][:go]
    json.ipr @result[uniprot_id][:ipr]

    json.taxon_id @result[uniprot_id][:taxon][:id]
    json.taxon_name @result[uniprot_id][:taxon][:name]
    json.taxon_rank @result[uniprot_id][:taxon][:rank]
  end
end
