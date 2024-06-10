class Api::Pept2protController < Api::ApiController
  before_action :set_cors_headers
  before_action :set_params
  # before_action :set_sequences
  before_action :search_input

  # Returns a list of Uniprot entries containing a given tryptic peptide
  # params[:input]: Array, required, List of input peptides
  # params[:equate_il]: "true" or "false" (default), optional, Equate I and L?
  # params[:extra]: "true" or "false" (default), optional, Output extra info?
  def pept2prot
    uniprot_accession_numbers = @sequences.map do |sequence| 
      sequence["uniprot_accession_numbers"]
    end.flatten.uniq

    uniprot_info = {}

    if @extra_info
      UniprotEntry
        .includes(:taxon, :ec_cross_references, :go_cross_references, :interpro_cross_references)
        .where(uniprot_accession_number: uniprot_accession_numbers)
        .each do |item| 
          uniprot_info[item.uniprot_accession_number] = {
            uniprot_id: item.uniprot_accession_number,
            name: item.name,
            taxon_id: item.taxon_id,
            protein: item.protein,
            taxon: item.taxon,
            ec_cross_references: item.ec_cross_references.map(&:ec_number_code).join(" "),
            go_cross_references: item.go_cross_references.map(&:go_term_code).join(" "),
            interpro_cross_references: item.interpro_cross_references.map(&:interpro_entry_code).join(" ")
          }
        end
    else
      UniprotEntry
        .where(uniprot_accession_number: uniprot_accession_numbers)
        .pluck(:uniprot_accession_number, :name, :taxon_id, :protein)
        .each do |item|
          uniprot_info[item[0]] = {
            uniprot_id: item[0],
            name: item[1],
            taxon_id: item[2],
            protein: item[3]
          }
        end
    end

    @result = {}
    @sequences.each do |sequence|
      sequence["uniprot_accession_numbers"].each do |uniprot_id|
        @result[sequence["sequence"]] = uniprot_info[uniprot_id]
      end
    end

    filter_input_order

    respond_with(@result)
  end
end
