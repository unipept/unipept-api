class PrivateApi::ProteinsController < PrivateApi::PrivateApiController
  include SuffixArrayHelper

  def proteins
    peptide = params[:peptide]
    equate_il = params[:equate_il].nil? ? true : params[:equate_il]

    unless params[:peptide]
      @error_name = 'Invalid peptide provided'
      @error_message = 'No peptide sequence was provided. Please provide a valid peptide sequence.'
      render 'private_api/error'
      return
    end

    unless peptide.length >= 5
      @error_name = 'Sequence too short'
      @error_message = 'The peptide sequence you provided is too short. It should contain at least 5 valid amino acids.'
      render 'private_api/error'
      return
    end

    puts peptide

    # Request the suffix array search service
    search_result = search([ peptide ], equate_il).first

    @lca = -1
    @common_lineage = []

    if search_result.nil?
      return
    end

    @lca = search_result["lca"]
    @proteins = UniprotEntry
      .includes(:taxon)
      .where(uniprot_accession_number: search_result["uniprot_accession_numbers"])
      .map do |protein| 
        {
          uniprot_accession_number: protein.uniprot_accession_number,
          name: protein.name,
          organism: protein.taxon_id,
          ec_numbers: protein.ec_cross_references.map(&:ec_number_code).reject(&:empty?),
          go_terms: protein.go_cross_references.map(&:go_term_code).reject(&:empty?),
          interpro_entries: protein.interpro_cross_references.map(&:interpro_entry_code).reject(&:empty?)
        }
      end

    # Lineage van de lca ophalen
    lca_lineage = Lineage.find(@lca)
    finished = (@lca == 1)
    while !finished && lca_lineage.has_next?
      next_rank = lca_lineage.next_t

      next if next_rank.nil?

      finished = (lca_lineage.id == next_rank.id)
      @common_lineage << next_rank.id
    end
  end
end
