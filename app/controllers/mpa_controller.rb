class MpaController < HandleOptionsController
  before_action :set_headers
  before_action :default_format_json
  skip_before_action :verify_authenticity_token, raise: false

  def pept2data
    peptides = params[:peptides] || []
    missed = params[:missed] || false
    @equate_il = params[:equate_il].nil? ? true : params[:equate_il]
    @peptides = Sequence
                .includes(Sequence.lca_t_relation_name(@equate_il) => :lineage)
                .where(sequence: peptides)
                .where.not(Sequence.lca_t_relation_name(@equate_il) => nil)
    if missed
      @peptides += peptides
                   .to_set.subtract(@peptides.map(&:sequence))
                   .map { |p| Sequence.missed_cleavage(p, @equate_il) }
                   .compact
    end

    @results_fa = {}
    @peptides.each do |sequence|
      @results_fa[sequence.sequence] = sequence.calculate_fa(@equate_il)
    end
  end

  def pept2filtered
    peptides = params[:peptides] || []
    # missed = params[:missed] || false
    taxa_filter_ids = (params[:taxa] || []).map(&:to_i)

    @equate_il = params[:equate_il].nil? ? true : params[:equate_il]

    @sequences = Sequence
                 .joins(peptides: [:uniprot_entry])
                 .includes(peptides: [:uniprot_entry])
                 .where(sequence: peptides)
                 .where(uniprot_entry: { taxon_id: taxa_filter_ids })
                 .uniq

    uniprot_ids = @sequences.map { |s| s.peptides.map(&:uniprot_entry_id) }.flatten.uniq

    @go_terms = GoCrossReference
                .where(uniprot_entry_id: uniprot_ids)

    @ec_numbers = EcCrossReference
                  .where(uniprot_entry_id: uniprot_ids)

    @ipr_entries = InterproCrossReference
                   .where(uniprot_entry_id: uniprot_ids)

    @seq_entries = @sequences.map { |s| [s, s.peptides.map(&:uniprot_entry).select { |e| taxa_filter_ids.include? e.taxon_id }] }
  end

  private

  def default_format_json
    request.format = 'json'
  end
end
