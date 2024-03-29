require 'test_helper'

class Api::Pept2interproControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get pept2interpro' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ipr":[]},
      {"peptide":"AAILER","total_protein_count":1,"ipr":[{"code":"IPR000169","protein_count":1}]}
    ]'

    get :pept2interpro, params: { input: %w[AAIER AAILER], format: 'json' }
  end

  test 'should get pept2interpro with il' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":22,"ipr":[{"code":"IPR000169","protein_count":1}]},
      {"peptide":"AAILER","total_protein_count":0,"ipr":[]}
    ]'

    get :pept2interpro, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
  end

  test 'should get pept2interpro with extra' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ipr":[]},
      {"peptide":"AAILER","total_protein_count":1,"ipr":[{"code":"IPR000169","protein_count":1,"name":"Cysteine peptidase, cysteine active site","type":"Active_site"}]}
    ]'

    get :pept2interpro, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
  end

  test 'should get pept2interpro with domains' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ipr":[]},
      {"peptide":"AAILER","total_protein_count":1,"ipr":[{"Active_site":[{"code":"IPR000169","protein_count":1}]}]}
    ]'

    get :pept2interpro, params: { input: %w[AAIER AAILER], format: 'json', domains: 'true' }
  end

  test 'should get pept2interpro with extra and domains' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ipr":[]},
      {"peptide":"AAILER","total_protein_count":1,"ipr":[{"Active_site":[{"code":"IPR000169","protein_count":1,"name":"Cysteine peptidase, cysteine active site"}]}]}
    ]'

    get :pept2interpro, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true', domains: 'true' }
  end

  test 'should get pept2interpro with extra and domains and il' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":22,"ipr":[{"Active_site":[{"code":"IPR000169","protein_count":1,"name":"Cysteine peptidase, cysteine active site"}]}]},
      {"peptide":"AAILER","total_protein_count":0,"ipr":[]}
    ]'

    get :pept2interpro, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true', extra: 'true', domains: 'true' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
