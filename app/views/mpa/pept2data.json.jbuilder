json.peptides @response do |peptide, lca|
  json.sequence peptide
  json.lca lca
  json.lineage @lineages[lca]
  json.fa do
    json.counts({ "all": 0, "EC": 0, "GO": 0, "IPR": 0 })
    json.data({})
  end
end
