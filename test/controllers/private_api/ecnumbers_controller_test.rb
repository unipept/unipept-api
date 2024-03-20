require 'test_helper'

class PrivateApi::EcnumbersControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get ecnumbers' do
    @expected = '[
      {"code":"1.2.3.4","name":"Some Enzyme"},
      {"code":"2.7.11.1","name":"Some Enzyme2"}
    ]'

    get :ecnumbers, params: { ecnumbers: %w[1.2.3.4 2.7.11.1] }
  end

  test 'should get ecnumbers no match' do
    @expected = '[]'

    get :ecnumbers, params: { ecnumbers: %w[x.x.x.x] }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
