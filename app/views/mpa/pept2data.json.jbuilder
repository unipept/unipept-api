json.peptides @response do |peptide, lca|
  json.sequence peptide
  json.lca lca
  json.lineage @lineages[lca]
  json.fa do
    json.counts({})
    json.data({})
  end
end
