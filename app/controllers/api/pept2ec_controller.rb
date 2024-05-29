class Api::Pept2ecController < Api::ApiController
  include FunctionalityHelper

  before_action :set_headers
  before_action :set_params

  # Returns the functional EC numbers for a given tryptic peptide
  # param[input]: Array, required, List of input peptides
  # param[equate_il]: "true" of "false", Indicate if you want to equate I and L
  # param[extra]: "true" or "false", optional, Output extra info?
  def pept2ec
    @result = pept2ec_helper
  end
end
