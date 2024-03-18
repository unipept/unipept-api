require 'test_helper'

class Api::Pept2goControllerTest < ActionController::TestCase
  test 'should get pept2go' do
    get :pept2go, params: { input: %w[AAIER AAILER], format: 'json' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'peptide'
    assert @response.body.include? 'total_protein_count'
    assert @response.body.include? 'go'
    assert @response.body.include? 'go_term'
    assert @response.body.include? 'protein_count'
    assert_not @response.body.include? 'name'
    assert_not @response.body.include? 'molecular function'
    assert_not @response.body.include? 'biological process'
    assert_not @response.body.include? 'cellular component'
  end

  test 'should get pept2go with il' do
    get :pept2go, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'peptide'
    assert @response.body.include? 'total_protein_count'
    assert @response.body.include? 'go'
    assert @response.body.include? 'go_term'
    assert @response.body.include? 'protein_count'
    assert_not @response.body.include? 'name'
    assert_not @response.body.include? 'molecular function'
    assert_not @response.body.include? 'biological process'
    assert_not @response.body.include? 'cellular component'
  end

  test 'should get pept2go with extra' do
    get :pept2go, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'peptide'
    assert @response.body.include? 'total_protein_count'
    assert @response.body.include? 'go'
    assert @response.body.include? 'go_term'
    assert @response.body.include? 'protein_count'
    assert @response.body.include? 'name'
    assert_not @response.body.include? 'molecular function'
    assert_not @response.body.include? 'biological process'
    assert_not @response.body.include? 'cellular component'
  end

  test 'should get pept2go with domains' do
    get :pept2go, params: { input: %w[AAIER AAILER], format: 'json', domains: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'peptide'
    assert @response.body.include? 'total_protein_count'
    assert @response.body.include? 'go'
    assert @response.body.include? 'go_term'
    assert @response.body.include? 'protein_count'
    assert_not @response.body.include? 'name'
    assert @response.body.include? 'molecular function'
    assert @response.body.include? 'biological process'
    assert @response.body.include? 'cellular component'
  end

  test 'should get pept2go with extra and domains' do
    get :pept2go, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true', domains: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'peptide'
    assert @response.body.include? 'total_protein_count'
    assert @response.body.include? 'go'
    assert @response.body.include? 'go_term'
    assert @response.body.include? 'protein_count'
    assert @response.body.include? 'name'
    assert @response.body.include? 'molecular function'
    assert @response.body.include? 'biological process'
    assert @response.body.include? 'cellular component'
  end
end
