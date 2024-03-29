require 'test_helper'

class Api::Pept2functControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get pept2funct' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[],"go":[{"go_term":"GO:0051301","protein_count":3},{"go_term":"GO:0005525","protein_count":3},{"go_term":"GO:0046872","protein_count":3},{"go_term":"GO:0007049","protein_count":3}],"ipr":[]},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1}],"go":[{"go_term":"GO:0005576","protein_count":1},{"go_term":"GO:0000287","protein_count":1},{"go_term":"GO:0004634","protein_count":1},{"go_term":"GO:0000015","protein_count":1},{"go_term":"GO:0006096","protein_count":1},{"go_term":"GO:0009986","protein_count":1}],"ipr":[{"code":"IPR000169","protein_count":1}]}
    ]'

    get :pept2funct, params: { input: %w[AAIER AAILER], format: 'json' }
  end

  test 'should get pept2funct with il' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":22,"ec":[{"ec_number":"2.7.11.1","protein_count":4}],"go":[{"go_term":"GO:0004674","protein_count":1},{"go_term":"GO:0005634","protein_count":1},{"go_term":"GO:0005524","protein_count":1},{"go_term":"GO:0016301","protein_count":1}],"ipr":[{"code":"IPR000169","protein_count":1}]},
      {"peptide":"AAILER","total_protein_count":0,"ec":[],"go":[],"ipr":[]}
    ]'
    
    get :pept2funct, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
  end

  test 'should get pept2funct with extra' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[],"go":[{"go_term":"GO:0051301","protein_count":3,"name":"some function 9"},{"go_term":"GO:0005525","protein_count":3,"name":"some function 10"},{"go_term":"GO:0046872","protein_count":3,"name":"some function 11"},{"go_term":"GO:0007049","protein_count":3,"name":"some function 12"}],"ipr":[]},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1,"name":"Some Enzyme3"}],"go":[{"go_term":"GO:0005576","protein_count":1,"name":"some function 16"},{"go_term":"GO:0000287","protein_count":1,"name":"some function 17"},{"go_term":"GO:0004634","protein_count":1,"name":"some function 18"},{"go_term":"GO:0000015","protein_count":1,"name":"some function 19"},{"go_term":"GO:0006096","protein_count":1,"name":"some function 20"},{"go_term":"GO:0009986","protein_count":1,"name":"some function 21"}],"ipr":[{"code":"IPR000169","protein_count":1,"name":"Cysteine peptidase, cysteine active site","type":"Active_site"}]}
    ]'
    
    get :pept2funct, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
  end

  test 'should get pept2funct with domains' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[],"go":[{"molecular function":[{"go_term":"GO:0051301","protein_count":3},{"go_term":"GO:0007049","protein_count":3}]},{"cellular component":[{"go_term":"GO:0005525","protein_count":3}]},{"biological process":[{"go_term":"GO:0046872","protein_count":3}]}],"ipr":[]},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1}],"go":[{"cellular component":[{"go_term":"GO:0005576","protein_count":1},{"go_term":"GO:0009986","protein_count":1}]},{"molecular function":[{"go_term":"GO:0000287","protein_count":1},{"go_term":"GO:0000015","protein_count":1}]},{"biological process":[{"go_term":"GO:0004634","protein_count":1},{"go_term":"GO:0006096","protein_count":1}]}],"ipr":[{"Active_site":[{"code":"IPR000169","protein_count":1}]}]}
    ]'
    
    get :pept2funct, params: { input: %w[AAIER AAILER], format: 'json', domains: 'true' }
  end

  test 'should get pept2funct with extra and domains' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[],"go":[{"molecular function":[{"go_term":"GO:0051301","protein_count":3,"name":"some function 9"},{"go_term":"GO:0007049","protein_count":3,"name":"some function 12"}]},{"cellular component":[{"go_term":"GO:0005525","protein_count":3,"name":"some function 10"}]},{"biological process":[{"go_term":"GO:0046872","protein_count":3,"name":"some function 11"}]}],"ipr":[]},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1,"name":"Some Enzyme3"}],"go":[{"cellular component":[{"go_term":"GO:0005576","protein_count":1,"name":"some function 16"},{"go_term":"GO:0009986","protein_count":1,"name":"some function 21"}]},{"molecular function":[{"go_term":"GO:0000287","protein_count":1,"name":"some function 17"},{"go_term":"GO:0000015","protein_count":1,"name":"some function 19"}]},{"biological process":[{"go_term":"GO:0004634","protein_count":1,"name":"some function 18"},{"go_term":"GO:0006096","protein_count":1,"name":"some function 20"}]}],"ipr":[{"Active_site":[{"code":"IPR000169","protein_count":1,"name":"Cysteine peptidase, cysteine active site"}]}]}
    ]'
    
    get :pept2funct, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true', domains: 'true' }
  end

  test 'should get pept2funct with extra and domains and il' do
    @expected = '[
      {"peptide":"AAIER","total_protein_count":22,"ec":[{"ec_number":"2.7.11.1","protein_count":4,"name":"Some Enzyme2"}],"go":[{"biological process":[{"go_term":"GO:0004674","protein_count":1,"name":"some function 5"},{"go_term":"GO:0016301","protein_count":1,"name":"some function 8"}]},{"molecular function":[{"go_term":"GO:0005634","protein_count":1,"name":"some function 6"}]},{"cellular component":[{"go_term":"GO:0005524","protein_count":1,"name":"some function 7"}]}],"ipr":[{"Active_site":[{"code":"IPR000169","protein_count":1,"name":"Cysteine peptidase, cysteine active site"}]}]},
      {"peptide":"AAILER","total_protein_count":0,"ec":[],"go":[],"ipr":[]}
    ]'
    
    get :pept2funct, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true', extra: 'true', domains: 'true' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
