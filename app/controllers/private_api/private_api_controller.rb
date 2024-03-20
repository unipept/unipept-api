class PrivateApi::PrivateApiController < HandleOptionsController
  before_action :set_headers
  before_action :default_format_json
  skip_before_action :verify_authenticity_token, raise: false

  private

  def default_format_json
    request.format = 'json'
  end
end
