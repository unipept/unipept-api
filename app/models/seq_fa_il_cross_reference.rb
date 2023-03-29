class SeqFaIlCrossReference < ApplicationRecord
  include ReadOnlyModel

  belongs_to :sequence, foreign_key: 'seq_id', primary_key: 'id', class_name: 'Sequence'
end
