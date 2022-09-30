json.peptides @seq_entries do |seq_entry|
  json.sequence seq_entry[0].sequence
  json.taxa seq_entry[1].map(&:taxon_id)
end
