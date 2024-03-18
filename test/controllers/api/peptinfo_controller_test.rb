require 'test_helper'

class Api::PeptinfoControllerTest < ActionController::TestCase
  test 'should get peptinfo' do
    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json' }
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
    assert @response.body.include? 'ec'
    assert @response.body.include? 'ec_number'
    assert_not @response.body.include? 'some function 2'
    assert_not @response.body.include? 'molecular function'
    assert_not @response.body.include? 'biological process'
    assert_not @response.body.include? 'cellular component'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get peptinfo with il' do
    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
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
    assert @response.body.include? 'ec'
    assert @response.body.include? 'ec_number'
    assert_not @response.body.include? 'some function 2'
    assert_not @response.body.include? 'molecular function'
    assert_not @response.body.include? 'biological process'
    assert_not @response.body.include? 'cellular component'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get peptinfo with extra' do
    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
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
    assert @response.body.include? 'ec'
    assert @response.body.include? 'ec_number'
    assert @response.body.include? 'some function 2'
    assert_not @response.body.include? 'molecular function'
    assert_not @response.body.include? 'biological process'
    assert_not @response.body.include? 'cellular component'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get peptinfo with domains' do
    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json', domains: 'true' }
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
    assert @response.body.include? 'ec'
    assert @response.body.include? 'ec_number'
    assert_not @response.body.include? 'some function 2'
    assert @response.body.include? 'molecular function'
    assert @response.body.include? 'biological process'
    assert @response.body.include? 'cellular component'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert_not @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end

  test 'should get peptinfo with extra and domains' do
    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true', domains: 'true' }
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
    assert @response.body.include? 'ec'
    assert @response.body.include? 'ec_number'
    assert @response.body.include? 'some function 2'
    assert @response.body.include? 'molecular function'
    assert @response.body.include? 'biological process'
    assert @response.body.include? 'cellular component'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert @response.body.include? 'taxon_rank'
    assert @response.body.include? 'kingdom_id'
    assert_not @response.body.include? 'kingdom_name'
  end
end
