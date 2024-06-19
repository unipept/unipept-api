json.array! @input_order do |peptide|
  if @result.key? peptide
    json.peptide peptide
    json.total_protein_count @result[peptide][:total]
    json.ec(@result[peptide][:ec].sort_by { |v| -v[:protein_count] })
  end
end
