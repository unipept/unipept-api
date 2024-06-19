require 'test_helper'

class Api::Pept2ecControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get pept2ec' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[]},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1}]}
    ]'

    get :pept2ec, params: { input: %w[AAIER AAILER], format: 'json' }
  end

  test 'should get pept2ec with il' do
    stub_http_request! 'test/fixtures/index/response_equate.json'

    @expected = '[
      {"peptide":"AAIER","total_protein_count":22,"ec":[{"ec_number":"2.7.11.1","protein_count":4}]},
      {"peptide":"AAILER","total_protein_count":0,"ec":[]}
    ]'
    
    get :pept2ec, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
  end

  test 'should get pept2ec with extra' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[]},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1,"name":"Some Enzyme3"}]}
    ]'
    
    get :pept2ec, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
  end

  test 'should get pept2ec with extra and il' do
    stub_http_request! 'test/fixtures/index/response_equate.json'

    @expected = '[
      {"peptide":"AAIER","total_protein_count":22,"ec":[{"ec_number":"2.7.11.1","protein_count":4,"name":"Some Enzyme2"}]},
      {"peptide":"AAILER","total_protein_count":0,"ec":[]}
    ]'
    
    get :pept2ec, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true', extra: 'true' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
