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

    # Request the suffix array search service
    search_result = search([ peptide ], equate_il).first

    @lca = search_result["lca"] || -1
    @common_lineage = []

    if search_result.nil?
      return
    end

    @proteins = UniprotEntry
      .includes(:taxon)
      .where(uniprot_accession_number: search_result["uniprot_accessions"])
      .map do |protein| 
        annotations = protein.fa.split(";")
        {
          uniprot_accession_number: protein.uniprot_accession_number,
          name: protein.name,
          organism: protein.taxon_id,
          ec_numbers: annotations.filter { |a| a.start_with? "EC" },
          go_terms: annotations.filter { |a| a.start_with? "GO" },
          interpro_entries: annotations.filter { |a| a.start_with? "IPR" }
        }
      end

    # TODO: Common lineage calculation
  end
end
