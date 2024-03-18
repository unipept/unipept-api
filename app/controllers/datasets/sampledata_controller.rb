class Datasets::SampledataController < Datasets::DatasetsController
  def sampledata
    @datasets = Dataset.includes(:dataset_items).all
  end
end
