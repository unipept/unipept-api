json.peptides @seq_entries do |seq_entry|
  json.sequence seq_entry[0].sequence
  json.taxa seq_entry[1].map(&:taxon_id)
  json.fa do
    json.go_terms @go_terms.map(&:go_term_code).uniq
    json.ec_numbers(@ec_numbers.map(&:ec_number_code).uniq.map { |ec| "EC:#{ec}" })
    json.interpro_entries(@ipr_entries.map(&:interpro_entry_code).uniq.map { |ipr| ipr.sub('IPR', 'IPR:') })
  end
end
