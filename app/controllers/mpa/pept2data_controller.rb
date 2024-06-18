class Mpa::Pept2dataController < Mpa::MpaController
  include SuffixArrayHelper

  def get_time
    Process.clock_gettime(Process::CLOCK_MONOTONIC, :millisecond)
  end

  def pept2data
    start = get_time

    peptides = params[:peptides] || []
    missed = params[:missed].nil? ? false : params[:missed]
    equate_il = params[:equate_il].nil? ? true : params[:equate_il]

    @response = Hash.new
    @lineages = Hash.new

    if peptides.empty?
      return
    end

    # Request the suffix array search service
    @response = search(peptides, equate_il).uniq

    # Collect all lca's to look up the lineages
    taxa = []
    @response.each do |result|
      taxa.append(result["lca"])
    end

    # Retrieve all lineages at once
    Lineage.find(taxa).each do |lineage|
      @lineages[lineage.taxon_id] = lineage.to_a_idx
    end

    @timings = Hash.new
    @timings["total"] = get_time - start
  end
end
