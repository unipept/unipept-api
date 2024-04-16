class Mpa::Pept2dataController < Mpa::MpaController
  include SuffixArrayHelper

  def pept2data
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
  end
end
