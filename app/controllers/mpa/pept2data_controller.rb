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

    @lineages = Hash.new
    Lineage.find(taxa).each do |lineage|
      @lineages[lineage.taxon_id] = lineage.to_a_idx
    end

    @response.each do |_, value|
      value["lineage"] = @lineages[value["lca"]]
    end

    # proteins = Set.new
    # search_results["result"].each do |result|
    #   proteins.merge(result['uniprot_accessions'])
    # end

    # entries = UniprotEntry.where(uniprot_accession_number: proteins.to_a.uniq)

    # # Convert the retrieved entries to a hash (for easy retrieval)
    # accession_to_protein = Hash.new
    # entries.each do |entry|
    #   accession_to_protein[entry.uniprot_accession_number] = entry
    # end

    # taxa = []
    # search_results["result"].each do |result|
    #   uniprot_entries = result["uniprot_accessions"].map { |acc| accession_to_protein[acc] }
    #   result["fa"] = UniprotEntry.summarize_fa(uniprot_entries)
    #   @response[result["sequence"]] = result
    #   taxa.append(result["lca"])
    # end

    # looked_up_lineages = Lineage.find(taxa)
    # looked_up_lineages.each do |lineage|
    #   @lineages[lineage.taxon_id] = lineage.to_a_idx
    # end

    # @response.each do |_, entry|
    #   entry["lineage"] = @lineages[entry["lca"].to_i]
    # end
  end
end
