class Api::Pept2protController < Api::ApiController
  before_action :set_cors_headers
  before_action :set_params
  before_action :set_sequences

  # Returns a list of Uniprot entries containing a given tryptic peptide
  # params[:input]: Array, required, List of input peptides
  # params[:equate_il]: "true" or "false" (default), optional, Equate I and L?
  # params[:extra]: "true" or "false" (default), optional, Output extra info?
  def pept2prot
    lookup = Hash.new { |h, k| h[k] = Set.new }
    if @extra_info
      @result = {}
      # Perform joins and load objects (expensive)
      ids = []
      @sequences.pluck(:sequence, 'uniprot_entries.id').each do |sequence, uniprot_id|
        ids.append uniprot_id
        lookup[uniprot_id] << sequence
        @result[sequence] = Set.new
      end

      ids = ids.uniq.compact.sort
      UniprotEntry.includes(:taxon, :ec_cross_references, :go_cross_references, :interpro_cross_references)
                  .where(id: ids).find_in_batches do |group|
        group.each do |uni|
          lookup[uni.id].each { |s| @result[s] << uni }
        end
      end
    else
      @result = Hash.new { |h, k| h[k] = Set.new }
      @sequences.pluck(:sequence, 'uniprot_entries.uniprot_accession_number', 'uniprot_entries.name', 'uniprot_entries.taxon_id', 'uniprot_entries.protein').each do |sequence, uniprot_id, protein_name, taxon_id, protein|
        @result[sequence] << [uniprot_id, protein_name, taxon_id, protein]
      end
    end

    filter_input_order

    respond_with(@result)
  end
end
