json.array! @input_order do |peptide|
  if @result.key? peptide
    json.peptide peptide
    json.partial! partial: 'api/pept2lca', locals: { data: @result[peptide] }
  end
end
