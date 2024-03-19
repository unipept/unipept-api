require 'test_helper'

class Api::ProtinfoControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get protinfo' do
    @expected = '[
      {"protein":"nr","ec":[{"ec_number":"1.2.3.4"}],"go":[],"ipr":[{"code":"IPR000126"}],"taxon_id":1,"taxon_name":"species1","taxon_rank":"species"},
      {"protein":"nr2","ec":[],"go":[],"ipr":[],"taxon_id":2,"taxon_name":"kingdom1","taxon_rank":"kingdom"}
    ]'

    get :protinfo, params: { input: %w[nr nr2], format: 'json' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
