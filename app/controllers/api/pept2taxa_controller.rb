class Api::Pept2taxaController < Api::ApiController
  before_action :set_headers
  before_action :set_params
  before_action :set_query

  # Returns a list of taxa retrieved from the Uniprot entries containing a given tryptic peptide
  # param[input]: Array, required, List of input peptides
  # param[equate_il]: "true" or "false", Indicate if you want to equate I and L
  # param[extra]: "true" or "false", Include lineage
  # param[names]: "true" or "false", Include the lineage names
  def pept2taxa
    @result = {}
    lookup = Hash.new { |h, k| h[k] = Set.new }
    ids = Set.new

    seqid2seq = {}
    Sequence.where(sequence: @input).select(:id, :sequence).each do |seq|
      seqid2seq[seq[:id]] = seq[:sequence]
      @result[seq[:sequence]] = Set.new
    end

    rel_name = @equate_il ? :sequence_id : :original_sequence_id
    Peptide.where(rel_name => seqid2seq.keys).select(:id, rel_name, :uniprot_entry_id).find_in_batches do |items|
      uniprot2seqids = Hash.new { |h, k| h[k] = [] }
      items.each { |i| uniprot2seqids[i[:uniprot_entry_id]] << i[rel_name] }

      UniprotEntry.where(id: uniprot2seqids.keys).select(:id, :taxon_id).each do |entry|
        uniprot2seqids[entry[:id]].each do |seqid|
          sequence = seqid2seq[seqid]
          lookup[entry[:taxon_id]] << sequence
          ids << entry[:taxon_id]
        end
      end
    end

    ids.delete nil
    ids = ids.to_a.sort

    @query.where(id: ids).find_in_batches do |group|
      group.each do |t|
        lookup[t.id].each { |s| @result[s] << t }
      end
    end

    filter_input_order

    respond_with(@result)
  end
end
