require 'test_helper'

class Api::Taxa2lcaControllerTest < ActionController::TestCase
  test 'should get taxa2lca' do
    get :taxa2lca, params: { input: %w[3 2], format: 'json' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get taxa2lca with root' do
    get :taxa2lca, params: { input: %w[1 2], format: 'json' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get taxa2lca with extra' do
    get :taxa2lca, params: { input: %w[1 2], format: 'json', extra: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get taxa2lca with names' do
    get :taxa2lca, params: { input: %w[1 2], format: 'json', names: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get taxa2lca with extra and names' do
    get :taxa2lca, params: { input: %w[1 2], format: 'json', names: 'true', extra: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert @response.body.include? 'kingdom_id'
    assert @response.body.include? 'kingdom_name'
  end
end
