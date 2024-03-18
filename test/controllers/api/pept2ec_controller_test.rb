require 'test_helper'

class Api::Pept2ecControllerTest < ActionController::TestCase
  test 'should get pept2ec' do
    get :pept2ec, params: { input: %w[AAIER AAILER], format: 'json' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'peptide'
    assert @response.body.include? 'total_protein_count'
    assert @response.body.include? 'ec'
    assert @response.body.include? 'ec_number'
    assert @response.body.include? 'protein_count'
    assert_not @response.body.include? 'name'
  end

  test 'should get pept2ec with il' do
    get :pept2ec, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'peptide'
    assert @response.body.include? 'total_protein_count'
    assert @response.body.include? 'ec'
    assert @response.body.include? 'ec_number'
    assert @response.body.include? 'protein_count'
    assert_not @response.body.include? 'name'
  end

  test 'should get pept2ec with extra' do
    get :pept2ec, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'peptide'
    assert @response.body.include? 'total_protein_count'
    assert @response.body.include? 'ec'
    assert @response.body.include? 'ec_number'
    assert @response.body.include? 'protein_count'
    assert @response.body.include? 'name'
  end
end
