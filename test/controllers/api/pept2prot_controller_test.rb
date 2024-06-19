require 'test_helper'

class Api::Pept2protControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get pept2prot' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '[
      {"peptide":"AAIER","uniprot_id":"nr","protein_name":"some name","taxon_id":1,"protein":"ELABA"},
      {"peptide":"AAILER","uniprot_id":"nr3","protein_name":"some name","taxon_id":2,"protein":"AAILERAGGAR"},
      {"peptide":"AAILER","uniprot_id":"nr4","protein_name":"some name","taxon_id":1,"protein":"AAILERA"}
    ]'

    get :pept2prot, params: { input: %w[AAIER AAILER], format: 'json' }
  end

  test 'should get pept2prot with il' do
    stub_http_request! 'test/fixtures/index/response_equate.json'

    @expected = '[
      {"peptide":"AAIER","uniprot_id":"nr","protein_name":"some name","taxon_id":1,"protein":"ELABA"},
      {"peptide":"AAIER","uniprot_id":"nr2","protein_name":"some name","taxon_id":2,"protein":"EIABA"},
      {"peptide":"AAILER","uniprot_id":"nr3","protein_name":"some name","taxon_id":2,"protein":"AAILERAGGAR"},
      {"peptide":"AAILER","uniprot_id":"nr4","protein_name":"some name","taxon_id":1,"protein":"AAILERA"}
    ]'
    
    get :pept2prot, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
  end

  test 'should get pept2prot with extra' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '[
      {"peptide":"AAIER","uniprot_id":"nr","protein_name":"some name","taxon_id":1,"taxon_name":"species1","ec_references":"1.2.3.4","go_references":"goid","interpro_references":"IPR000126","protein":"ELABA"},
      {"peptide":"AAILER","uniprot_id":"nr3","protein_name":"some name","taxon_id":2,"taxon_name":"kingdom1","ec_references":"","go_references":"","interpro_references":"","protein":"AAILERAGGAR"},
      {"peptide":"AAILER","uniprot_id":"nr4","protein_name":"some name","taxon_id":1,"taxon_name":"species1","ec_references":"","go_references":"","interpro_references":"","protein":"AAILERA"}
    ]'
    
    get :pept2prot, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
  end

  test 'should get pept2prot with extra and il' do
    stub_http_request! 'test/fixtures/index/response_equate.json'

    @expected = '[
      {"peptide":"AAIER","uniprot_id":"nr","protein_name":"some name","taxon_id":1,"taxon_name":"species1","ec_references":"1.2.3.4","go_references":"goid","interpro_references":"IPR000126","protein":"ELABA"},
      {"peptide":"AAIER","uniprot_id":"nr2","protein_name":"some name","taxon_id":2,"taxon_name":"kingdom1","ec_references":"","go_references":"","interpro_references":"","protein":"EIABA"},
      {"peptide":"AAILER","uniprot_id":"nr3","protein_name":"some name","taxon_id":2,"taxon_name":"kingdom1","ec_references":"","go_references":"","interpro_references":"","protein":"AAILERAGGAR"},
      {"peptide":"AAILER","uniprot_id":"nr4","protein_name":"some name","taxon_id":1,"taxon_name":"species1","ec_references":"","go_references":"","interpro_references":"","protein":"AAILERA"}
    ]'
    
    get :pept2prot, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true', extra: 'true' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
