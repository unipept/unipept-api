require 'test_helper'

class PrivateApi::GotermsControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get goterms' do
    @expected = '[
      {"code":"GO:0012345","name":"some function","namespace":"cellular component"},
      {"code":"GO:0016569","name":"some function 2","namespace":"cellular component"}
    ]'

    get :goterms, params: { goterms: %w[GO:0012345 GO:0016569] }
  end

  test 'should get goterms no match' do
    @expected = '[]'

    get :goterms, params: { goterms: %w[GO:xxxxxxx] }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
