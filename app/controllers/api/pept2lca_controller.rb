class Api::Pept2lcaController < Api::ApiController
  include TaxonomyHelper

  before_action :set_cors_headers
  before_action :set_params
  before_action :set_query
  before_action :search_input

  # Returns the taxonomic lowest common ancestor for a given tryptic peptide
  # param[input]: Array, required, List of input peptides
  # param[equate_il]: "true" or "false", Indicate if you want to equate I and L
  # param[extra]: "true" or "false", Include lineage
  # param[names]: "true" or "false", Include the lineage names
  def pept2lca
    @result = pept2lca_helper
    filter_input_order
  end
end
