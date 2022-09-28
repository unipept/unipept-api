# == Schema Information
#
# Table name: interpro_cross_references
#
#  id                  :integer          unsigned, not null, primary key
#  uniprot_entry_id    :integer          unsigned, not null
#  interpro_entry_code :string(9)        not null
#

class InterproCrossReference < ApplicationRecord
  include ReadOnlyModel

  belongs_to :uniprot_entry
  belongs_to :interpro_entry, foreign_key: 'interpro_entry_code', primary_key: 'code', class_name: 'InterproEntry'
end
