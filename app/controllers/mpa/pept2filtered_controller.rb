class Mpa::Pept2filteredController < Mpa::MpaController
  include SuffixArrayHelper

  def get_time
    Process.clock_gettime(Process::CLOCK_MONOTONIC, :millisecond)
  end

  def pept2filtered
    peptides = (params[:peptides] || []).uniq
    missed = params[:missed].nil? ? false : params[:missed]
    equate_il = params[:equate_il].nil? ? true : params[:equate_il]
    cutoff = params[:cutoff] || 1000

    @response = Hash.new

    if peptides.empty?
      return
    end

    taxa_filter_ids = (params[:taxa] || []).map(&:to_i)

    index_time = get_time

    # Request the suffix array search service
    @response = search(peptides, equate_il, cutoff)
      .select { |result| !result["cutoff_used"] }
      .each { |result| result["taxa"] = result["taxa"].to_set }

    end_index_time = get_time - index_time

    taxa_filter_ids.each_slice(5000) do |taxa_slice|
      taxa_slice = Taxon.where(id: taxa_slice).where(valid_taxon: 1).pluck(:id).to_set

      next if taxa_slice.empty?

      @response.each do |result|
        result["taxa"] = result["taxa"].select { |taxon_id| taxa_slice.include?(taxon_id) }.uniq
      end
    end

    @timings = end_index_time
  end
end
