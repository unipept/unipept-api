class PrivateApi::MetadataController < PrivateApi::PrivateApiController
  def metadata
    @data = {
      db_version: Rails.application.config.versions[:uniprot]
    }
  end
end
