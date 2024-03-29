require 'test_helper'

class Mpa::Pept2dataControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get pept2data' do
    @expected = '{
      "peptides":[
        {"sequence":"AAIER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":22,"EC":4,"GO":2,"IPR":3},"data":{"EC:2.7.11.1":4,"GO:0004674":1,"GO:0005634":1,"GO:0005524":1,"GO:0016301":1,"IPR:IPR000169":1}}},
        {"sequence":"AAILER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":0,"EC":0,"GO":0,"IPR":0},"data":{}}}
      ]
    }'

    get :pept2data, params: { peptides: %w[AAIER AAILER], format: 'json' }
  end

  test 'should get pept2data with il' do
    @expected = '{
      "peptides":[
        {"sequence":"AAIER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":22,"EC":4,"GO":2,"IPR":3},"data":{"EC:2.7.11.1":4,"GO:0004674":1,"GO:0005634":1,"GO:0005524":1,"GO:0016301":1,"IPR:IPR000169":1}}},
        {"sequence":"AAILER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":0,"EC":0,"GO":0,"IPR":0},"data":{}}}
      ]
    }'
    
    get :pept2data, params: { peptides: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
  end

  test 'should get pept2data with missed' do
    @expected = '{
      "peptides":[
        {"sequence":"AAIER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":22,"EC":4,"GO":2,"IPR":3},"data":{"EC:2.7.11.1":4,"GO:0004674":1,"GO:0005634":1,"GO:0005524":1,"GO:0016301":1,"IPR:IPR000169":1}}},
        {"sequence":"AAILER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":0,"EC":0,"GO":0,"IPR":0},"data":{}}}
      ]
    }'
    
    get :pept2data, params: { peptides: %w[AAIER AAILER], format: 'json', missed: 'true' }
  end

  test 'should get pept2data with missed and il' do
    @expected = '{
      "peptides":[
        {"sequence":"AAIER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":22,"EC":4,"GO":2,"IPR":3},"data":{"EC:2.7.11.1":4,"GO:0004674":1,"GO:0005634":1,"GO:0005524":1,"GO:0016301":1,"IPR:IPR000169":1}}},
        {"sequence":"AAILER","lca":1,"lineage":[null,2,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,null,1,null,null,null,null],"fa":{"counts":{"all":0,"EC":0,"GO":0,"IPR":0},"data":{}}}
      ]
    }'
    
    get :pept2data, params: { peptides: %w[AAIER AAILER], format: 'json', equate_il: 'true', missed: 'true' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
