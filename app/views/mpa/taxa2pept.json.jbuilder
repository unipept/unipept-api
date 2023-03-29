json.array! @taxa_seq do |taxon_id, seq_count|
  json.taxon_id taxon_id
  json.sequence_count seq_count
end
