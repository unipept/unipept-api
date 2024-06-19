json.array! @input_order do |peptide|
  if @result.key? peptide
    json.peptide peptide
    json.total_protein_count @result[peptide][:total]
    json.partial! partial: 'api/pept2interpro', locals: { data: @result[peptide][:ipr] }
  end
end
