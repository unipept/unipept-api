json.array! @input_order do |peptide|
  if @result.key? peptide
    json.peptide peptide
    json.total_protein_count @result[peptide][:total]
    json.ec(@result[peptide][:ec].sort_by { |value| -value[:protein_count] })
    json.partial! partial: 'api/pept2go', locals: { data: @result[peptide][:go] }
    json.partial! partial: 'api/pept2interpro', locals: { data: @result[peptide][:ipr] }
    json.partial! partial: 'api/pept2lca', locals: { data: @result[peptide][:lca] }
  end
end
