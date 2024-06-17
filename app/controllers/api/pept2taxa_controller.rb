class Api::Pept2taxaController < Api::ApiController
  include TaxonomyHelper

  before_action :set_cors_headers
  before_action :set_params
  before_action :set_query
  before_action :search_input

  # Returns a list of taxa retrieved from the Uniprot entries containing a given tryptic peptide
  # param[input]: Array, required, List of input peptides
  # param[equate_il]: "true" or "false", Indicate if you want to equate I and L
  # param[extra]: "true" or "false", Include lineage
  # param[names]: "true" or "false", Include the lineage names
  def pept2taxa
    @result = pept2taxa_helper
    filter_input_order
  end
end
