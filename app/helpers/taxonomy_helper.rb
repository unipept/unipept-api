module TaxonomyHelper
  def pept2lca_helper
    output = {}
    lookup = Hash.new { |h, k| h[k] = Set.new }
    ids = []
    @sequences = Sequence.where(sequence: @input)
    lca_field = @equate_il ? :lca_il : :lca
    @sequences.pluck(:sequence, lca_field).each do |sequence, lca_il|
      ids.append lca_il
      lookup[lca_il] << sequence
    end

    ids = ids.uniq.compact.sort

    @query.where(id: ids).find_in_batches do |group|
      group.each do |t|
        lookup[t.id].each { |s| output[s] = t }
      end
    end

    output
  end
end
