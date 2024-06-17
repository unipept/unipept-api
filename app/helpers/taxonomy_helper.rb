module TaxonomyHelper
  include SuffixArrayHelper

  def pept2lca_helper
    output = {}
    lookup = Hash.new { |h, k| h[k] = Set.new }
    ids = []

    @sequences.each do |seq|
      ids.append seq["lca"]
      lookup[seq["lca"]] << seq["sequence"]
    end

    ids = ids.uniq.compact.sort

    @query.where(id: ids).find_in_batches do |group|
      group.each do |t|
        lookup[t.id].each { |s| output[s] = t }
      end
    end

    output
  end

  def pept2taxa_helper
    output = Hash.new { |h, k| h[k] = Set.new }
    lookup = Hash.new { |h, k| h[k] = Set.new }
    ids = []

    @sequences.each do |seq|
      seq["taxa"].each do |taxon|
        ids.append taxon
        lookup[taxon] << seq["sequence"]
      end
    end

    ids = ids.uniq.compact.sort

    @query.where(id: ids).find_in_batches do |group|
      group.each do |t|
        lookup[t.id].each { |s| output[s] << t }
      end
    end

    output
  end
end
