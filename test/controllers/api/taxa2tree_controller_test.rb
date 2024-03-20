require 'test_helper'

class Api::Taxa2treeControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get taxa2tree' do
    @expected = '{
      "id":1,"name":"Organism","rank":"no rank","data":{"count":1,"self_count":0},"children":[
        {"id":13,"name":"kingdom2","rank":"kingdom","data":{"count":1,"self_count":0},"children":[
          {"id":14,"name":"phylum1","rank":"phylum","data":{"count":1,"self_count":1},"children":[]}
        ]}
      ]
    }'

    get :taxa2tree, params: { input: %w[14], format: 'json' }
  end

  test 'should get taxa2tree with names' do
    @expected = '{
      "id":1,"name":"Organism","rank":"no rank","data":{"count":1,"self_count":0},"children":[
        {"id":13,"name":"kingdom2","rank":"kingdom","data":{"count":1,"self_count":0},"children":[
          {"id":14,"name":"phylum1","rank":"phylum","data":{"count":1,"self_count":1},"children":[]}
        ]}
      ]
    }'

    get :taxa2tree, params: { input: %w[14], names: 'true', format: 'json' }
    puts @response.body
  end

  # test 'should get taxa2tree with extra' do
  #   @expected = '{
  #     "id":1,"name":"Organism","rank":"no rank","data":{"count":1,"self_count":0},"children":[
  #       {"id":13,"name":"kingdom2","rank":"kingdom","data":{"count":1,"self_count":0},"children":[
  #         {"id":14,"name":"phylum1","rank":"phylum","data":{"count":1,"self_count":1},"children":[]}
  #       ]}
  #     ]
  #   }'

  #   get :taxa2tree, params: { input: %w[14], extra: 'true', format: 'json' }
  # end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
