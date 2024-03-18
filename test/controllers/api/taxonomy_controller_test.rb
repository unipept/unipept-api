require 'test_helper'

class Api::TaxonomyControllerTest < ActionController::TestCase
  test 'should get taxonomy' do
    get :taxonomy, params: { input: %w[1 2], format: 'json' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get taxonomy with extra' do
    get :taxonomy, params: { input: %w[1 2], format: 'json', extra: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get taxonomy with names' do
    get :taxonomy, params: { input: %w[1 2], format: 'json', names: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get taxonomy with extra and names' do
    get :taxonomy, params: { input: %w[1 2], format: 'json', names: 'true', extra: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert @response.body.include? 'kingdom_id'
    assert @response.body.include? 'kingdom_name'
  end

  test "shouldn't crash when logging to stathat" do
    Rails.application.config.unipept_API_logging = true
    Rails.application.config.unipept_stathat_key = 'key'
    get :taxonomy, params: { input: %w[1 2], format: 'json' }
    assert_response :success
    Rails.application.config.unipept_API_logging = false
  end
end
