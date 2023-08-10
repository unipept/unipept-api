class MpaController < HandleOptionsController
  before_action :set_headers
  before_action :default_format_json
  skip_before_action :verify_authenticity_token, raise: false

  def pept2data
    peptides = params[:peptides] || []
    missed = params[:missed] || false
    @equate_il = params[:equate_il].nil? ? true : params[:equate_il]

    # If equate_il is set, we have to replace all I's by and L in the input peptides.
    equalized_pepts = @equate_il ? peptides.map { |p| p.gsub('I', 'L') } : peptides

    @peptides = Sequence
                .includes(Sequence.lca_t_relation_name(@equate_il) => :lineage)
                .where(sequence: equalized_pepts)
                .where.not(Sequence.lca_t_relation_name(@equate_il) => nil)
    if missed
      @peptides += equalized_pepts
                   .to_set.subtract(@peptides.map(&:sequence))
                   .map { |p| Sequence.missed_cleavage(p, @equate_il) }
                   .compact
    end

    eq_seq_to_fa = {}
    eq_seq_to_info = {}

    @peptides.each do |sequence|
      eq_seq_to_fa[sequence.sequence] = sequence.calculate_fa(@equate_il)
      eq_seq_to_info[sequence.sequence] = sequence
    end

    @original_pep_results = {}
    @original_pep_fas = {}

    peptides.each do |original_seq|
      equalized_seq = @equate_il ? original_seq.gsub('I', 'L') : original_seq
      if eq_seq_to_info.key? equalized_seq
        @original_pep_results[original_seq] = eq_seq_to_info[equalized_seq]
        @original_pep_fas[original_seq] = eq_seq_to_fa[equalized_seq]
      end
    end
  end

  def pept2filtered
    peptides = params[:peptides] || []
    cutoff = params[:cutoff] || 1000
    # missed = params[:missed] || false
    taxa_filter_ids = (params[:taxa] || []).map(&:to_i)

    @equate_il = params[:equate_il].nil? ? true : params[:equate_il]

    @seq_entries = {}
    uniprot_ids = []

    peptides_under_cutoff = Sequence
      .joins(:peptides)
      .where(sequence: peptides)
      .group('sequences.id')
      .having('count(peptides.id) < ?', cutoff)
      .pluck(:sequence)

    taxa_filter_ids.each_slice(5000) do |taxa_slice|
      sequence_subset = Sequence
                        .joins(peptides: [:uniprot_entry])
                        .includes(peptides: [:uniprot_entry])
                        .where(sequence: peptides_under_cutoff)
                        .where(uniprot_entry: { taxon_id: taxa_slice })
                        .uniq

      sequence_subset.each do |seq_info|
        @seq_entries[seq_info.sequence] = [] unless @seq_entries.key?(seq_info.sequence)

        @seq_entries[seq_info.sequence] += seq_info
                                           .peptides
                                           .map(&:uniprot_entry)
                                           .select { |e| taxa_filter_ids.include? e.taxon_id }

        @seq_entries[seq_info.sequence].uniq!
      end

      uniprot_ids += sequence_subset.map { |s| s.peptides.map(&:uniprot_entry_id) }.flatten.uniq
    end

    uniprot_ids = uniprot_ids.uniq

    @go_terms = GoCrossReference
                .where(uniprot_entry_id: uniprot_ids)

    @ec_numbers = EcCrossReference
                  .where(uniprot_entry_id: uniprot_ids)

    @ipr_entries = InterproCrossReference
                   .where(uniprot_entry_id: uniprot_ids)
  end

  private

  def default_format_json
    request.format = 'json'
  end
end
