class Api::ProtinfoController < Api::ApiController
  before_action :set_headers
  before_action :set_params

  # Returns the taxonomic and functional information for given uniprot id's
  # param[input]: Array, required, List of input uniprot id's
  def protinfo
    @result = {}

    UniprotEntry
      .includes(:taxon, :ec_numbers, :go_terms, :interpro_entries)
      .where(uniprot_accession_number: @input_order)
      .find_in_batches do |batch|
        batch.each do |uniprot_id|
          @result[uniprot_id.uniprot_accession_number] = {
            taxon: uniprot_id.taxon,
            ec: uniprot_id.ec_numbers.map { |ec| { ec_number: ec.code } },
            go: uniprot_id.go_terms.map { |go| { go_term: go.code } },
            ipr: uniprot_id.interpro_entries.map { |interpro| { code: interpro.code } }
          }
        end
      end

    respond_with(@result)
  end
end
