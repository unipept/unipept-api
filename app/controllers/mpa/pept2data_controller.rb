class Mpa::Pept2dataController < Mpa::MpaController
  include SuffixArrayHelper

  def pept2data
    peptides = params[:peptides] || []
    missed = params[:missed] || false
    equate_il = params[:equate_il].nil? ? true : params[:equate_il] == 'true'

    @response = Hash.new
    @lineages = Hash.new

    if peptides.empty?
      return
    end

    # Request the suffix array search service
    search_results = search(peptides, equate_il)

    # Collect all lca's to look up the lineages
    taxa = []
    search_results["result"].each do |result|
      taxa.append(result["lca"])
    end

    # Retrieve all lineages at once
    Lineage.find(taxa).each do |lineage|
      @lineages[lineage.taxon_id] = lineage.to_a_idx
    end

    @response = search_results["result"]
  end
end
