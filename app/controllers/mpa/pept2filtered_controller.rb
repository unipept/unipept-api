class Mpa::Pept2filteredController < Mpa::MpaController
  include SuffixArrayHelper

  def pept2filtered
    peptides = params[:peptides] || []
    missed = params[:missed].nil? ? false : params[:missed]
    equate_il = params[:equate_il].nil? ? true : params[:equate_il]
    cutoff = params[:cutoff] || 1000

    @response = Hash.new

    if peptides.empty?
      return
    end

    taxa_filter_ids = (params[:taxa] || []).map(&:to_i)

    # Request the suffix array search service
    @response = search(peptides, equate_il, cutoff).select { |result| !result["cutoff_used"] }.uniq

    # TODO: we should remove this or use a different approach
    @response.each do |result|
      result["taxa"] = result["taxa"].select { |taxon_id| taxa_filter_ids.include?(taxon_id) }.uniq
    end
  end
end
