class PrivateApi::TaxaController < PrivateApi::PrivateApiController
  def taxa
    taxids = params[:taxids] || []
    @taxa = Taxon.includes(:lineage).where(id: taxids)
  end
end
