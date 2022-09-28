class DatasetsController < HandleOptionsController
  before_action :set_headers, only: %i[sampledata]
  before_action :default_format_json, only: %i[sampledata]
  before_action :authorize, only: %i[new edit create update destroy]

  def sampledata
    @datasets = Dataset.includes(:dataset_items).all
  end

  private

  def default_format_json
    request.format = 'json'
  end
end
