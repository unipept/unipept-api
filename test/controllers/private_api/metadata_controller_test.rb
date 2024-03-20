require 'test_helper'

class PrivateApi::MetadataControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get metadata' do
    @expected = "{\"db_version\":\"#{Rails.application.config.versions[:uniprot]}\"}"

    get :metadata
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
