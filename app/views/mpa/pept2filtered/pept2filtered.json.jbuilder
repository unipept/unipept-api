json.peptides @response do |peptide|
  json.sequence peptide["sequence"]
  json.taxa peptide["taxa"]
  json.fa do
    json.counts peptide["fa"]["counts"]
    json.data peptide["fa"]["data"]
  end
end
json.timings @timings
