require 'test_helper'

class Api::Pept2lcaControllerTest < ActionController::TestCase
  test 'should get pept2lca' do
    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
    assert @response.body.include? 'AAIER","taxon_id":2'
  end

  test 'should get pept2lca with il' do
    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
    assert @response.body.include? 'AAIER","taxon_id":1'
  end

  test 'should get pept2lca with extra' do
    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
    assert @response.body.include? 'AAIER","taxon_id":2'
  end

  test 'should get pept2lca with names' do
    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json', names: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
    assert @response.body.include? 'AAIER","taxon_id":2'
  end

  test 'should get pept2lca with extra and names' do
    get :pept2lca, params: { input: %w[AAIER AAILER], extra: 'true', names: 'true' }, format: 'json'
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert @response.body.include? 'kingdom_id'
    assert @response.body.include? 'kingdom_name'
    assert @response.body.include? 'AAIER","taxon_id":2'
  end

  test 'should get pept2lca with extra and names and il' do
    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true', extra: 'true', names: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert @response.body.include? 'kingdom_id'
    assert @response.body.include? 'kingdom_name'
    assert @response.body.include? 'AAIER","taxon_id":1'
  end
end
