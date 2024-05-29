class Api::Pept2interproController < Api::ApiController
  include FunctionalityHelper

  before_action :set_headers
  before_action :set_params

  # Returns the functional interpro entries for given tryptic peptides
  # param[input]: Array, required, List of input peptides
  # param[equate_il]: "true" of "false", Indicate if you want to equate I and L
  # param[extra]: "true" or "false", optional, Output extra info?
  # param[domains]: "true" or "false", optional, Should InterPro entries be split according to type?
  def pept2interpro
    @result = pept2interpro_helper
  end
end
