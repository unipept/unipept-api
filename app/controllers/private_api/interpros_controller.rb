class PrivateApi::InterprosController < PrivateApi::PrivateApiController
  def interpros
    interpro_entries = params[:interpros]
    @interpros = InterproEntry.where(code: interpro_entries)
  end
end
