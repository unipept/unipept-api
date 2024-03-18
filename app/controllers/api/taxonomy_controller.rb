class Api::TaxonomyController < Api::ApiController
  before_action :set_headers
  before_action :set_params
  before_action :set_query
  
  # Returns the taxonomic information for a given list of taxon id's
  # param[input]: Array, required, List of input taxon ids
  # param[extra]: "true" or "false", Include lineage
  # param[names]: "true" or "false", Include the lineage names
  def taxonomy
    @result = @query.where(id: @input)
    @result = @result.index_by(&:id)
    @input_order = @input.select { |i| @result.key? i.to_i }
    @result = @input_order.map { |i| @result[i.to_i] }
    respond_with(@result)
  end
end
