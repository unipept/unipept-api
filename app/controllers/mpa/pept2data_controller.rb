class Mpa::Pept2dataController < Mpa::MpaController
  include SuffixArrayHelper

  def get_time
    Process.clock_gettime(Process::CLOCK_MONOTONIC, :millisecond)
  end

  def pept2data
    peptides = params[:peptides] || []
    missed = params[:missed] || false
    equate_il = params[:equate_il].nil? ? true : params[:equate_il] == 'true'

    @response = Hash.new

    search_time = get_time

    # Request the suffix array search service
    search_results = search(peptides, equate_il)

    search_time = get_time - search_time

    collect_taxa_time = get_time

    # Collect all lca's to look up the lineages
    taxa = []
    search_results["result"].each do |result|
      taxa.append(result["lca"])
    end

    collect_taxa_time = get_time - collect_taxa_time

    collect_lineage_time = get_time

    # Retrieve all lineages at once
    @lineages = Hash.new
    Lineage.find(taxa).each do |lineage|
      @lineages[lineage.taxon_id] = lineage.to_a_idx
    end

    collect_lineage_time = get_time - collect_lineage_time

    construct_response_time = get_time

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

    construct_response_time = get_time - construct_response_time

    @timings = Hash.new
    @timings["search"] = search_time
    @timings["collect_taxa"] = collect_taxa_time
    @timings["collect_lineage"] = collect_lineage_time
    @timings["construct_response"] = construct_response_time
  end
end
