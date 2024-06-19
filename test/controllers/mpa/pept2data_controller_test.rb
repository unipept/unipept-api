require 'test_helper'

class Mpa::Pept2dataControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get pept2data' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '{
      "peptides":[
        {"sequence":"AAILER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":1,"EC":1,"GO":1,"IPR":1},"data":{"EC:4.2.1.11": 1,"GO:0005576": 1,"GO:0000287": 1,"GO:0004634": 1,"GO:0000015": 1,"GO:0006096": 1,"GO:0009986": 1,"IPR:IPR000169": 1}}},
        {"sequence":"AAIER","lca":2,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null],"fa":{"counts":{"all":3,"EC":0,"GO":3,"IPR":0},"data":{"GO:0051301":3,"GO:0005525":3,"GO:0046872":3,"GO:0007049":3}}}
      ]
    }'

    get :pept2data, params: { peptides: %w[AAIER AAILER], format: 'json' }
  end

  test 'should get pept2data with il' do
    stub_http_request! 'test/fixtures/index/response_equate.json'

    @expected = '{
      "peptides":[
        {"sequence":"AAILER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":0,"EC":0,"GO":0,"IPR":0},"data":{}}},
        {"sequence":"AAIER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":22,"EC":4,"GO":2,"IPR":3},"data":{"EC:2.7.11.1":4,"GO:0004674":1,"GO:0005634":1,"GO:0005524":1,"GO:0016301":1,"IPR:IPR000169":1}}}
      ]
    }'
    
    get :pept2data, params: { peptides: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
