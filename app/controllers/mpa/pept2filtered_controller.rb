class Mpa::Pept2filteredController < Mpa::MpaController
  include SuffixArrayHelper

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

    # Request the suffix array search service
    @response = search(peptides, equate_il, cutoff)
      .select { |result| !result["cutoff_used"] }

    taxa_filter_ids = taxa_filter_ids.to_set

    @response.each do |result|
      result[:taxa] = result[:taxa].to_set & taxa_filter_ids
    end

    @response.reject! { |result| result[:taxa].empty? }
  end
end
