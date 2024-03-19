class Mpa::Pept2dataController < Mpa::MpaController
  def pept2data
    peptides = params[:peptides] || []
    missed = params[:missed] || false
    @equate_il = params[:equate_il].nil? ? true : params[:equate_il] == 'true'

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
end
