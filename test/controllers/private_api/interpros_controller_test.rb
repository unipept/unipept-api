require 'test_helper'

class PrivateApi::InterprosControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get interpros' do
    @expected = '[
      {"code":"IPR000126","category":"Active_site","name":"Serine proteases, V8 family, serine active site"},
      {"code":"IPR000169","category":"Active_site","name":"Cysteine peptidase, cysteine active site"}
    ]'

    get :interpros, params: { interpros: %w[IPR000126 IPR000169] }
  end

  test 'should get interpros no match' do
    @expected = '[]'

    get :interpros, params: { interpros: %w[IPRxxxxx] }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
