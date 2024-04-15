class Mpa::Pept2dataController < Mpa::MpaController
  include SuffixArrayHelper

  def pept2data
    peptides = params[:peptides] || []
    missed = params[:missed] || false
    @equate_il = params[:equate_il].nil? ? true : params[:equate_il] == 'true'

    @response = Hash.new

    # Request the suffix array search service
    search_results = search(peptides, @equate_il)

    taxa = []
    search_results["result"].each do |result|
      @response[result["sequence"]] = {
        sequence: result["sequence"],
        lca: result["lca"],
        fa: result["fa"]
      }
      taxa.append(result["lca"])
    end

    lineages = Hash.new
    Lineage.find(taxa).each do |lineage|
      lineages[lineage.taxon_id] = lineage.to_a_idx
    end

    @response.each do |_, value|
      value["lineage"] = @lineages[value["lca"].to_i]
    end
  end
end
