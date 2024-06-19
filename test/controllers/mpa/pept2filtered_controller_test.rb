require 'test_helper'

class Mpa::Pept2filteredControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get pept2filtered' do
    stub_http_request! 'test/fixtures/index/response.json'
    
    @expected = '{
      "peptides":[
        {"sequence":"AAILER","taxa":[1, 2],"fa":{"counts":{"GO":1, "EC":1, "all":1, "IPR":1}, "data":{"EC:4.2.1.11":1, "GO:0005576":1, "GO:0000287":1, "GO:0004634":1, "GO:0000015":1, "GO:0006096":1, "GO:0009986":1, "IPR:IPR000169":1}}},
        {"sequence":"AAIER","taxa":[1],"fa":{"counts":{"IPR":0, "EC":0, "all":3, "GO":3}, "data":{"GO:0051301":3, "GO:0005525":3, "GO:0046872":3, "GO:0007049":3}}}
      ]
    }'

    get :pept2filtered, params: { peptides: %w[AAIER AAILER], taxa: %w[1 2 13 14 15], format: 'json' }
  end

  test 'should get pept2filtered with il' do
    stub_http_request! 'test/fixtures/index/response_equate.json'
    
    @expected = '{
      "peptides":[
        {"sequence":"AAILER","taxa":[1, 2],"fa":{"counts":{"GO":0, "EC":0, "all":0, "IPR":0}, "data":{}}},
        {"sequence":"AAIER","taxa":[1, 2],"fa":{"counts":{"IPR":3, "EC":4, "all":22, "GO":2}, "data":{"EC:2.7.11.1":4, "GO:0004674":1, "GO:0005634":1, "GO:0005524":1, "GO:0016301":1, "IPR:IPR000169":1}}}
      ]
    }'

    get :pept2filtered, params: { peptides: %w[AAIER AAILER], taxa: %w[1 2 13 14 15], format: 'json', equate_il: 'true' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
