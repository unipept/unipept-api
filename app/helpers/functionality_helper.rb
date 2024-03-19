module FunctionalityHelper
  def pept2ec_helper
    output = {}

    @sequences = Sequence.where(sequence: @input)

    ec_numbers = []

    @sequences.each do |seq|
      fa = seq.calculate_fa(@equate_il)
      # TODO: this ['num'] is a bug and should be removed before merging
      #ecs = fa['num']['data'].select { |k, _v| k.start_with?('EC:') }
      ecs = fa['data'].select { |k, _v| k.start_with?('EC:') }

      output[seq.sequence] = {
        total: fa['num']['all'],
        ec: ecs.map do |k, v|
              {
                ec_number: k[3..],
                protein_count: v
              }
            end
      }

      ec_numbers.push(*(ecs.map { |k, _v| k[3..] }))
    end

    if @extra_info
      ec_numbers = ec_numbers.uniq.compact.sort

      ec_mapping = {}

      EcNumber.where(code: ec_numbers).each do |ec_term|
        ec_mapping[ec_term.code] = ec_term.name
      end

      output.each do |_k, v|
        v[:ec].each do |value|
          value[:name] = ec_mapping[value[:ec_number]]
        end
      end
    end

    output
  end

  def pept2go_helper
    output = {}
    go_terms = []

    @sequences = Sequence.where(sequence: @input)

    @sequences.each do |seq|
      fa = seq.calculate_fa(@equate_il)
      # TODO: this ['num'] is a bug and should be removed before merging
      #gos = fa['num']['data'].select { |k, _v| k.start_with?('GO:') }
      gos = fa['data'].select { |k, _v| k.start_with?('GO:') }

      output[seq.sequence] = {
        total: fa['num']['all'],
        go: gos.map do |k, v|
              {
                go_term: k,
                protein_count: v
              }
            end
      }

      go_terms.push(*gos.keys)
    end

    if @extra_info || @domains
      go_terms = go_terms.uniq.compact.sort

      go_mapping = {}
      GoTerm.where(code: go_terms).each do |go_term|
        go_mapping[go_term.code] = go_term
      end

      if @domains

        set_name = if @extra_info
                     ->(value) { value[:name] = go_mapping[value[:go_term]].name }
                   else
                     # Do nothing
                     ->(_value) {}
                   end

        # We have to transform the input so that the different GO-terms are split per namespace
        output.each do |_k, v|
          splitted = Hash.new { |h, k1| h[k1] = [] }

          v[:go].each do |value|
            go_term = go_mapping[value[:go_term]]
            set_name[value]
            splitted[go_term.namespace] << value
          end

          v[:go] = splitted
        end
      else
        output.map do |_k, v|
          v[:go].each do |value|
            value[:name] = go_mapping[value[:go_term]].name
          end
        end
      end
    end

    output
  end

  def pept2interpro_helper
    output = {}
    ipr_entries = []

    @sequences = Sequence.where(sequence: @input)

    @sequences.each do |seq|
      fa = seq.calculate_fa(@equate_il)
      # TODO: this ['num'] is a bug and should be removed before merging
      #iprs = fa['num']['data'].select { |k, _v| k.start_with?('IPR:') }
      iprs = fa['data'].select { |k, _v| k.start_with?('IPR:') }

      output[seq.sequence] = {
        total: fa['num']['all'],
        ipr: iprs.map do |k, v|
               {
                 code: k[4..],
                 protein_count: v
               }
             end
      }

      ipr_entries.push(*(iprs.map { |k, _v| k[4..] }))
    end

    if @extra_info || @domains
      ipr_entries = ipr_entries.uniq.compact.sort
      ipr_mapping = {}

      InterproEntry.where(code: ipr_entries).each do |ipr_entry|
        ipr_mapping[ipr_entry.code] = ipr_entry
      end

      if @domains
        # We have to transform the input so that the different InterPro entries are split per type
        output.each do |_k, v|
          splitted = Hash.new { |h, k1| h[k1] = [] }

          v[:ipr].each do |value|
            ipr_entry = ipr_mapping[value[:code]]

            unless ipr_entry.nil?
              value[:name] = ipr_entry.name if @extra_info
              splitted[ipr_entry.category] << value
            end
          end

          v[:ipr] = splitted
        end
      else
        output.map do |_k, v|
          v[:ipr].each do |value|
            ipr_entry = ipr_mapping[value[:code]]
            value[:name] = ipr_entry.nil? ? '' : ipr_entry.name
            value[:type] = ipr_entry.nil? ? '' : ipr_entry.category
          end
        end
      end
    end

    output
  end
end
  