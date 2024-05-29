class Datasets::DatasetsController < HandleOptionsController
  before_action :default_format_json
  before_action :set_cors_headers

  private

  def default_format_json
    request.format = 'json'
  end
end
