class Api::Pept2goController < Api::ApiController
  include FunctionalityHelper

  before_action :set_cors_headers
  before_action :set_params
  before_action :search_input

  # Returns the functional GO terms for a given tryptic peptide
  # param[input]: Array, required, List of input peptides
  # param[equate_il]: "true" of "false", Indicate if you want to equate I and L
  # param[extra]: "true" or "false", optional, Output extra info?
  # param[domains]: "true" or "false", optional, Should GO_terms be split according to namespace?
  def pept2go
    @result = pept2go_helper
  end
end
