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

    end_index_time = get_time - index_time

    filter_time = get_time

    taxa_filter_ids = taxa_filter_ids.to_set

    @response.each do |result|
      result["taxa"] = result["taxa"].to_set  & taxa_filter_ids
    end

    end_filter_time = get_time - filter_time

    @timings = {
      index_time: end_index_time,
      filter_time: end_filter_time
    }
  end
end
