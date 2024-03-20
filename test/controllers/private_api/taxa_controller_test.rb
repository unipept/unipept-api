require 'test_helper'

class PrivateApi::TaxaControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get taxa' do
    @expected = '[
      {"id":1,"name":"species1","rank":"species","lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null]},
      {"id":13,"name":"kingdom2","rank":"kingdom","lineage":[null,13,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null]}
    ]'

    get :taxa, params: { taxids: %w[1 13] }
  end

  test 'should get taxa no match' do
    @expected = '[]'

    get :taxa, params: { taxids: %w[52] }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
