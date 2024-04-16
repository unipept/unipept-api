json.peptides @response do |peptide|
  json.sequence peptide[:sequence]
  json.lca peptide[:lca]
  json.lineage @lineages[peptide[:lca].to_i]
  json.fa peptide[:fa]
end
