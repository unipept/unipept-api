json.peptides @original_pep_results do |orig_seq, peptide|
  json.sequence orig_seq
  json.lca @equate_il ? peptide.lca_il : peptide.lca
  l = peptide.send(Sequence.lca_t_relation_name(@equate_il)).lineage
  json.lineage(Lineage.ranks.map { |rank| l.send(rank) })
  json.fa do
    json.counts @original_pep_fas[orig_seq]['num']
    json.data @original_pep_fas[orig_seq]['data']
  end
end
