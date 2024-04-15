# == Schema Information
#
# Table name: uniprot_entries
#
#  id                       :integer          unsigned, not null, primary key
#  uniprot_accession_number :string(10)       not null
#  version                  :integer          unsigned, not null
#  taxon_id                 :integer          unsigned, not null
#  type                     :string(9)        not null
#  name                     :string(150)      not null
#  protein                  :text(65535)      not null
#  fa                       :text(65535)      not null

class UniprotEntry < ApplicationRecord
  include ReadOnlyModel

  belongs_to :taxon,
             primary_key: 'id',
             class_name: 'Taxon'

  belongs_to :lineage,          foreign_key: 'taxon_id',
                                primary_key: 'taxon_id',
                                class_name: 'Lineage'

  # the type attribute is used by rails to specify inheritance so we change
  # the default value
  self.inheritance_column = 'type_id'

  def protein_contains?(sequence, equate_il)
    if equate_il
      protein.tr('I', 'L').include? sequence.tr('I', 'L')
    else
      protein.include? sequence
    end
  end

  # Summarises the functional annotations of a list of entries
  # Note: this should only be used for peptides who's FA's have
  # not been precalculated (because they were mot in the DB)
  #
  # @param entries list of UniprotEntries that match the sequence
  # In order to speed up this query, it's a good idea to include
  # the cross references already when requesting all UniprotEntry's
  # with a specific id.
  def self.summarize_fa(entries)
    data = Hash.new(0)

    ups_with_go = Set.new
    ups_with_ec = Set.new
    ups_with_ipr = Set.new

    entries.each do |uniprot_entry|
      uniprot_entry.fa.split(";").each do |fa, count|
        # TODO: These checks can be improved once the fa-object of a protein has been expanded
        if fa.start_with? "GO"
          ups_with_go.add(uniprot_entry.id)
        end

        if fa.start_with? "EC"
          ups_with_ec.add(uniprot_entry.id)
        end

        if fa.start_with? "IPR"
          ups_with_ipr.add(uniprot_entry.id)
        end

        data[fa] += 1
      end
    end

    {
      'num' => {
        'all' => entries.length,
        'EC' => ups_with_ec.length,
        'GO' => ups_with_go.length,
        'IPR' => ups_with_ipr.length
      },
      'data' => data
    }
  end
end
