require 'test_helper'

class Api::Pept2protControllerTest < ActionController::TestCase
  test 'set_params should parse single peptide input correctly' do
    get :pept2prot, params: { input: 'AAIER', format: 'json' }
    assert_equal ['AAIER'], assigns(:input)
    assert_not assigns(:equate_il)
    assert_not assigns(:extra_info)
    assert_not assigns(:names)
  end

  test 'set_params should parse hash input correctly' do
    get :pept2prot, params: { input: { 0 => 'AAIER' }, format: 'json' }
    assert_equal ['AAIER'], assigns(:input)
  end

  test 'set_params should parse array input correctly' do
    get :pept2prot, params: { input: %w[AAIER TEST], format: 'json' }
    assert_equal %w[AAIER TEST], assigns(:input)
  end

  test 'set_params should parse json input correctly' do
    get :pept2prot, params: { input: '["AAIER", "TEST"]', format: 'json' }
    assert_equal %w[AAIER TEST], assigns(:input)
  end

  test 'set_params should parse boolean options correctly' do
    get :pept2prot, params: { input: 'AAIER', format: 'json', equate_il: 'true', extra: 'true', names: 'true' }
    assert_equal ['AALER'], assigns(:input)
    assert_equal ['AAIER'], assigns(:input_order)
    assert assigns(:equate_il)
    assert assigns(:extra_info)
    assert assigns(:names)
  end

  test 'should get pept2prot' do
    get :pept2prot, params: { input: %w[AAIER AAILER], format: 'json' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'uniprot_id'
    assert @response.body.include? 'protein_name'
    assert @response.body.include? 'taxon_id'
    assert_not @response.body.include? 'taxon_name'
    assert_not @response.body.include? '"uniprot_id":"nr2"'
  end

  test 'should get pept2prot with il' do
    get :pept2prot, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'uniprot_id'
    assert @response.body.include? 'protein_name'
    assert @response.body.include? 'taxon_id'
    assert_not @response.body.include? 'taxon_name'
    assert @response.body.include? '"uniprot_id":"nr2"'
  end

  test 'should get pept2prot with extra' do
    get :pept2prot, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'uniprot_id'
    assert @response.body.include? 'protein_name'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert_not @response.body.include? '"taxon_name":null'
    assert @response.body.include? 'ec_references'
    assert @response.body.include? 'go_references'
    assert_not @response.body.include? '"uniprot_id":"nr2"'
  end

  test 'should get pept2prot with extra and il' do
    get :pept2prot, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true', extra: 'true' }
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert @response.body.include? 'AAIER'
    assert @response.body.include? 'AAILER'
    assert_not @response.body.include? 'AALLER'
    assert_not @response.body.include? 'AALER'
    assert @response.body.include? 'uniprot_id'
    assert @response.body.include? 'protein_name'
    assert @response.body.include? 'taxon_id'
    assert @response.body.include? 'taxon_name'
    assert_not @response.body.include? '"taxon_name":null'
    assert @response.body.include? 'ec_references'
    assert @response.body.include? 'go_references'
    assert @response.body.include? '"uniprot_id":"nr2"'
  end
end
