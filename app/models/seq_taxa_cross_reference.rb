# frozen_string_literal: true

class SeqTaxaCrossReference < ApplicationRecord
  include ReadOnlyModel

  belongs_to :sequence, foreign_key: 'seq_id', primary_key: 'id', class_name: 'Sequence'
  belongs_to :taxon, foreign_key: 'taxon_id', primary_key: 'id', class_name: 'Taxon'
end
