class Api::Pept2functController < Api::ApiController
  include FunctionalityHelper

  before_action :set_cors_headers
  before_action :set_params
  before_action :search_input

  # Returns the functional GO terms and EC numbers for a given tryptic peptide
  # param[input]: Array, required, List of input peptides
  # param[equate_il]: "true" or "false", Indicate if you want to equate I and L
  # param[extra]: "true" or "false", optional, Output extra info?
  # param[domains]: "true" or "false", optional, Should GO_terms be split according to namespace?
  def pept2funct
    @result = {}

    ec_result = pept2ec_helper
    go_result = pept2go_helper
    interpro_result = pept2interpro_helper

    @input_order.each do |seq|
      seq_index = @equate_il ? seq.tr('I', 'L') : seq

      next unless go_result.key? seq_index

      @result[seq_index] = {
        total: go_result[seq_index][:total],
        go: go_result[seq_index][:go],
        ec: ec_result[seq_index][:ec],
        ipr: interpro_result[seq_index][:ipr]
      }
    end
  end
end
