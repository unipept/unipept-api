json.peptides @seq_entries.each do |sequence, uniprot_entries|
  json.sequence sequence
  json.taxa uniprot_entries.map(&:taxon_id).uniq
  json.fa do
    json.go_terms(@go_terms
                  .select { |go| uniprot_entries.map(&:id).include? go.uniprot_entry_id }
                  .map(&:go_term_code)
                  .reject(&:empty?)
                  .uniq)

    json.ec_numbers(@ec_numbers
                    .select { |ec| uniprot_entries.map(&:id).include? ec.uniprot_entry_id }
                    .map(&:ec_number_code)
                    .reject(&:empty?)
                    .uniq
                    .map { |ec| "EC:#{ec}" })

    json.interpro_entries(@ipr_entries
                          .select { |ipr| uniprot_entries.map(&:id).include? ipr.uniprot_entry_id }
                          .map(&:interpro_entry_code)
                          .reject(&:empty?)
                          .uniq
                          .map { |ipr| ipr.sub('IPR', 'IPR:') })
  end
end
