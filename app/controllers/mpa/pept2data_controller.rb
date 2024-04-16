class Mpa::Pept2dataController < Mpa::MpaController
  include SuffixArrayHelper

  def pept2data
    peptides = params[:peptides] || []
    missed = params[:missed] || false
    equate_il = params[:equate_il].nil? ? true : params[:equate_il] == 'true'

    @response = Hash.new

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
    @lineages = Hash.new
    Lineage.find(taxa).each do |lineage|
      @lineages[lineage.taxon_id] = lineage.to_a_idx
    end

    # Fill the response hash with the search results
    search_results["result"].each do |result|
      @response[result["sequence"]] = {
        sequence: result["sequence"],
        lca: result["lca"],
        lineage: @lineages[result["lca"].to_i],
        fa: {
          counts: result["fa"]["counts"],
          data: result["fa"]["data"]
        }
      }
    end
  end
end
