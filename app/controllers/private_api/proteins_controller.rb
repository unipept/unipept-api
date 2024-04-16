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
    search_result = search([ peptide ], equate_il)
    if search_result.empty?
      return
    end

    # Collect all protein information
    @proteins = UniprotEntry.where(uniprot_accession_number: search_result[0]["uniprot_accessions"])

    @lca = search_result["lca"] || -1


    # if sequence.present? && sequence.peptides(equate_il).empty?
    #   @entries = []
    #   return
    # end

    # @common_lineage = []

    # # get the uniprot entries of every peptide
    # # only used for the open in uniprot links
    # # and calculate the LCA
    # if sequence.nil?
    #   begin
    #     # we didn't find the sequence in the database, so let's try to split it
    #     long_sequences = Sequence.advanced_single_search(seq, equate_il)
    #   rescue NoMatchesFoundError
    #     return
    #   end
    #   # calculate possible uniprot entries
    #   temp_entries = long_sequences.map { |s| s.peptides(equate_il).map(&:uniprot_entry).to_set }
    #   # take the intersection of all sets
    #   @entries = temp_entries.reduce(:&)
    #   # check if the protein contains the startsequence
    #   @entries.select! { |e| e.protein_contains?(seq, equate_il) }

    #   # Calculate fa summary
    #   @fa_summary = UniprotEntry.summarize_fa(@entries)

    #   return if @entries.empty?

    #   @lineages = @entries.map(&:lineage).compact
    # else
    #   @entries = sequence.peptides(equate_il).map(&:uniprot_entry)
    #   @lineages = sequence.lineages(equate_il, true).to_a

    #   # Get FA summary from cache
    #   @fa_summary = sequence.calculate_fa(equate_il)
    # end

    # # sort entries
    # @entries = @entries.to_a.sort_by { |e| e.taxon.nil? ? '' : e.taxon.name }

    # @lca_taxon = Lineage.calculate_lca_taxon(@lineages)
    # @root = Node.new(1, 'Organism', nil, 'root') # start constructing the tree
    # common_hits = @lineages.map(&:hits).reduce(:+)
    # @root.data['count'] = common_hits
    # last_node = @root

    # # common lineage
    # # construct the common lineage in this array
    # l = @lca_taxon.lineage
    # found = (@lca_taxon.name == 'root')
    # while !found && l.has_next?
    #   t = l.next_t
    #   next if t.nil?

    #   found = (@lca_taxon.id == t.id)
    #   @common_lineage << t
    #   node = Node.new(t.id, t.name, @root, t.rank)
    #   node.data['count'] = common_hits
    #   last_node = last_node.add_child(node)
    # end
  end
end
