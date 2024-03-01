require 'net/http'
require 'uri'
require 'json'

class MpaController < HandleOptionsController
  before_action :set_headers
  before_action :default_format_json
  skip_before_action :verify_authenticity_token, raise: false

  def pept2data
    peptides = params[:peptides] || []
    missed = params[:missed] || false
    @equate_il = params[:equate_il].nil? ? true : params[:equate_il]

    if peptides.empty?
      @response = Hash.new
      @lineages = Hash.new

      return
    end

    # Convert the peptides array into a JSON string
    json_data = {peptides: peptides}.to_json

    # The URL to which the request will be sent
    uri = URI.parse("http://localhost:3000")

    # Create a POST request
    request = Net::HTTP::Post.new(uri)
    request.content_type = "application/json"
    request.body = json_data

    # Set up the HTTP session
    response = Net::HTTP.start(uri.hostname, uri.port) do |http|
      http.request(request)
    end

    # Parse the response body as JSON
    response_data = JSON.parse(response.body)

    # Initialize an empty Hash for the mapping
    @response = {}

    # Iterate over the 'result' array in the response data
    response_data["result"].each do |item|
      # Map each peptide sequence to its taxon_id in the Hash
      @response[item["peptide"]] = item["taxon_id"]
    end

    taxa = Taxon.includes(:lineage).find(@response.values)

    @lineages = Hash.new
    taxa.each do |taxon|
      @lineages[taxon.id] = taxon.lineage.to_a_idx
    end

    puts @lineages.inspect

    @response
  end

  def pept2filtered
    peptides = params[:peptides] || []
    cutoff = params[:cutoff] || 1000
    # missed = params[:missed] || false
    taxa_filter_ids = (params[:taxa] || []).map(&:to_i)

    @equate_il = params[:equate_il].nil? ? true : params[:equate_il]

    @seq_entries = {}
    uniprot_ids = []

    peptides_under_cutoff = Sequence
                            .joins(:peptides)
                            .where(sequence: peptides)
                            .group('sequences.id')
                            .having('count(peptides.id) < ?', cutoff)
                            .pluck(:sequence)

    taxa_filter_ids.each_slice(5000) do |taxa_slice|
      # For all given taxon id's, filter out those that are invalid according to Unipept's filter
      # i.e. these taxa are typically rubbish (for example those that end in bacterium and are classified at the
      # species rank).
      taxa_slice = Taxon
                   .where(id: taxa_slice)
                   .where(valid_taxon: 1)
                   .pluck(:id)

      # If none of the taxa in this slice are valid, skip this iteration of the loop and continue with the next one.
      next if taxa_slice.empty?

      sequence_subset = Sequence
                        .joins(peptides: [:uniprot_entry])
                        .includes(peptides: [:uniprot_entry])
                        .where(sequence: peptides_under_cutoff)
                        .where(uniprot_entry: { taxon_id: taxa_slice })
                        .uniq

      sequence_subset.each do |seq_info|
        @seq_entries[seq_info.sequence] = [] unless @seq_entries.key?(seq_info.sequence)

        @seq_entries[seq_info.sequence] += seq_info
                                           .peptides
                                           .map(&:uniprot_entry)
                                           .select { |e| taxa_filter_ids.include? e.taxon_id }

        @seq_entries[seq_info.sequence].uniq!
      end

      uniprot_ids += sequence_subset.map { |s| s.peptides.map(&:uniprot_entry_id) }.flatten.uniq
    end

    uniprot_ids = uniprot_ids.uniq

    @go_terms = GoCrossReference
                .where(uniprot_entry_id: uniprot_ids)

    @ec_numbers = EcCrossReference
                  .where(uniprot_entry_id: uniprot_ids)

    @ipr_entries = InterproCrossReference
                   .where(uniprot_entry_id: uniprot_ids)
  end

  private

  def default_format_json
    request.format = 'json'
  end
end
