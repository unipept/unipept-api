class Api::Taxa2lcaController < Api::ApiController
  before_action :set_headers
  before_action :set_params

  # Returns the lowest common ancestor for a given list of taxon id's
  # param[input]: Array, required, List of input taxon ids
  # param[extra]: "true" or "false", Include lineage
  # param[names]: "true" or "false", Include the lineage names
  def taxa2lca
    # handle case where 1 is provided
    if @input.include? '1'
      @result = Taxon.find(1)
    else
      lineages = Lineage.includes(Lineage::ORDER_T).where(taxon_id: @input)
      @result = Lineage.calculate_lca_taxon(lineages)
    end

    respond_with(@result)
  end
end
  