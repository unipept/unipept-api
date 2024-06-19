require 'test_helper'

class Api::PeptinfoControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get peptinfo' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[],"go":[{"go_term":"GO:0051301","protein_count":3},{"go_term":"GO:0005525","protein_count":3},{"go_term":"GO:0046872","protein_count":3},{"go_term":"GO:0007049","protein_count":3}],"ipr":[],"taxon_id":2,"taxon_name":"kingdom1","taxon_rank":"kingdom"},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1}],"go":[{"go_term":"GO:0005576","protein_count":1},{"go_term":"GO:0000287","protein_count":1},{"go_term":"GO:0004634","protein_count":1},{"go_term":"GO:0000015","protein_count":1},{"go_term":"GO:0006096","protein_count":1},{"go_term":"GO:0009986","protein_count":1}],"ipr":[{"code":"IPR000169","protein_count":1}],"taxon_id":1,"taxon_name":"species1","taxon_rank":"species"}
    ]'

    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json' }
  end

  test 'should get peptinfo with il' do
    stub_http_request! 'test/fixtures/index/response_equate.json'
    
    @expected = '[
      {"peptide":"AAIER","total_protein_count":22,"ec":[{"ec_number":"2.7.11.1","protein_count":4}],"go":[{"go_term":"GO:0004674","protein_count":1},{"go_term":"GO:0005634","protein_count":1},{"go_term":"GO:0005524","protein_count":1},{"go_term":"GO:0016301","protein_count":1}],"ipr":[{"code":"IPR000169","protein_count":1}],"taxon_id":1,"taxon_name":"species1","taxon_rank":"species"},
      {"peptide":"AAILER","total_protein_count":0,"ec":[],"go":[],"ipr":[],"taxon_id":1,"taxon_name":"species1","taxon_rank":"species"}
    ]'

    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
  end

  test 'should get peptinfo with extra' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[],"go":[{"go_term":"GO:0051301","protein_count":3,"name":"some function 9"},{"go_term":"GO:0005525","protein_count":3,"name":"some function 10"},{"go_term":"GO:0046872","protein_count":3,"name":"some function 11"},{"go_term":"GO:0007049","protein_count":3,"name":"some function 12"}],"ipr":[],"taxon_id":2,"taxon_name":"kingdom1","taxon_rank":"kingdom","superkingdom_id":null,"kingdom_id":2,"subkingdom_id":null,"superphylum_id":null,"phylum_id":null,"subphylum_id":null,"superclass_id":null,"class_id":null,"subclass_id":null,"infraclass_id":null,"superorder_id":null,"order_id":null,"suborder_id":null,"infraorder_id":null,"parvorder_id":null,"superfamily_id":null,"family_id":null,"subfamily_id":null,"tribe_id":null,"subtribe_id":null,"genus_id":null,"subgenus_id":null,"species_group_id":null,"species_subgroup_id":null,"species_id":null,"subspecies_id":null,"varietas_id":null,"forma_id":null},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1,"name":"Some Enzyme3"}],"go":[{"go_term":"GO:0005576","protein_count":1,"name":"some function 16"},{"go_term":"GO:0000287","protein_count":1,"name":"some function 17"},{"go_term":"GO:0004634","protein_count":1,"name":"some function 18"},{"go_term":"GO:0000015","protein_count":1,"name":"some function 19"},{"go_term":"GO:0006096","protein_count":1,"name":"some function 20"},{"go_term":"GO:0009986","protein_count":1,"name":"some function 21"}],"ipr":[{"code":"IPR000169","protein_count":1,"name":"Cysteine peptidase, cysteine active site","type":"Active_site"}],"taxon_id":1,"taxon_name":"species1","taxon_rank":"species","superkingdom_id":null,"kingdom_id":2,"subkingdom_id":null,"superphylum_id":null,"phylum_id":null,"subphylum_id":null,"superclass_id":null,"class_id":null,"subclass_id":null,"infraclass_id":null,"superorder_id":null,"order_id":null,"suborder_id":null,"infraorder_id":null,"parvorder_id":null,"superfamily_id":null,"family_id":null,"subfamily_id":null,"tribe_id":null,"subtribe_id":null,"genus_id":null,"subgenus_id":null,"species_group_id":null,"species_subgroup_id":null,"species_id":1,"subspecies_id":null,"varietas_id":null,"forma_id":null}
    ]'

    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
  end

  test 'should get peptinfo with domains' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[],"go":[{"molecular function":[{"go_term":"GO:0051301","protein_count":3},{"go_term":"GO:0007049","protein_count":3}]},{"cellular component":[{"go_term":"GO:0005525","protein_count":3}]},{"biological process":[{"go_term":"GO:0046872","protein_count":3}]}],"ipr":[],"taxon_id":2,"taxon_name":"kingdom1","taxon_rank":"kingdom"},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1}],"go":[{"cellular component":[{"go_term":"GO:0005576","protein_count":1},{"go_term":"GO:0009986","protein_count":1}]},{"molecular function":[{"go_term":"GO:0000287","protein_count":1},{"go_term":"GO:0000015","protein_count":1}]},{"biological process":[{"go_term":"GO:0004634","protein_count":1},{"go_term":"GO:0006096","protein_count":1}]}],"ipr":[{"Active_site":[{"code":"IPR000169","protein_count":1}]}],"taxon_id":1,"taxon_name":"species1","taxon_rank":"species"}
    ]'

    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json', domains: 'true' }
  end

  test 'should get peptinfo with extra and domains' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '[
      {"peptide":"AAIER","total_protein_count":3,"ec":[],"go":[{"molecular function":[{"go_term":"GO:0051301","protein_count":3,"name":"some function 9"},{"go_term":"GO:0007049","protein_count":3,"name":"some function 12"}]},{"cellular component":[{"go_term":"GO:0005525","protein_count":3,"name":"some function 10"}]},{"biological process":[{"go_term":"GO:0046872","protein_count":3,"name":"some function 11"}]}],"ipr":[],"taxon_id":2,"taxon_name":"kingdom1","taxon_rank":"kingdom","superkingdom_id":null,"kingdom_id":2,"subkingdom_id":null,"superphylum_id":null,"phylum_id":null,"subphylum_id":null,"superclass_id":null,"class_id":null,"subclass_id":null,"infraclass_id":null,"superorder_id":null,"order_id":null,"suborder_id":null,"infraorder_id":null,"parvorder_id":null,"superfamily_id":null,"family_id":null,"subfamily_id":null,"tribe_id":null,"subtribe_id":null,"genus_id":null,"subgenus_id":null,"species_group_id":null,"species_subgroup_id":null,"species_id":null,"subspecies_id":null,"varietas_id":null,"forma_id":null},
      {"peptide":"AAILER","total_protein_count":1,"ec":[{"ec_number":"4.2.1.11","protein_count":1,"name":"Some Enzyme3"}],"go":[{"cellular component":[{"go_term":"GO:0005576","protein_count":1,"name":"some function 16"},{"go_term":"GO:0009986","protein_count":1,"name":"some function 21"}]},{"molecular function":[{"go_term":"GO:0000287","protein_count":1,"name":"some function 17"},{"go_term":"GO:0000015","protein_count":1,"name":"some function 19"}]},{"biological process":[{"go_term":"GO:0004634","protein_count":1,"name":"some function 18"},{"go_term":"GO:0006096","protein_count":1,"name":"some function 20"}]}],"ipr":[{"Active_site":[{"code":"IPR000169","protein_count":1,"name":"Cysteine peptidase, cysteine active site"}]}],"taxon_id":1,"taxon_name":"species1","taxon_rank":"species","superkingdom_id":null,"kingdom_id":2,"subkingdom_id":null,"superphylum_id":null,"phylum_id":null,"subphylum_id":null,"superclass_id":null,"class_id":null,"subclass_id":null,"infraclass_id":null,"superorder_id":null,"order_id":null,"suborder_id":null,"infraorder_id":null,"parvorder_id":null,"superfamily_id":null,"family_id":null,"subfamily_id":null,"tribe_id":null,"subtribe_id":null,"genus_id":null,"subgenus_id":null,"species_group_id":null,"species_subgroup_id":null,"species_id":1,"subspecies_id":null,"varietas_id":null,"forma_id":null}
    ]'

    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true', domains: 'true' }
  end

  test 'should get peptinfo with extra and domains and il' do
    stub_http_request! 'test/fixtures/index/response_equate.json'

    @expected = '[
      {"peptide":"AAIER","total_protein_count":22,"ec":[{"ec_number":"2.7.11.1","protein_count":4,"name":"Some Enzyme2"}],"go":[{"biological process":[{"go_term":"GO:0004674","protein_count":1,"name":"some function 5"},{"go_term":"GO:0016301","protein_count":1,"name":"some function 8"}]},{"molecular function":[{"go_term":"GO:0005634","protein_count":1,"name":"some function 6"}]},{"cellular component":[{"go_term":"GO:0005524","protein_count":1,"name":"some function 7"}]}],"ipr":[{"Active_site":[{"code":"IPR000169","protein_count":1,"name":"Cysteine peptidase, cysteine active site"}]}],"taxon_id":1,"taxon_name":"species1","taxon_rank":"species","superkingdom_id":null,"kingdom_id":2,"subkingdom_id":null,"superphylum_id":null,"phylum_id":null,"subphylum_id":null,"superclass_id":null,"class_id":null,"subclass_id":null,"infraclass_id":null,"superorder_id":null,"order_id":null,"suborder_id":null,"infraorder_id":null,"parvorder_id":null,"superfamily_id":null,"family_id":null,"subfamily_id":null,"tribe_id":null,"subtribe_id":null,"genus_id":null,"subgenus_id":null,"species_group_id":null,"species_subgroup_id":null,"species_id":1,"subspecies_id":null,"varietas_id":null,"forma_id":null},
      {"peptide":"AAILER","total_protein_count":0,"ec":[],"go":[],"ipr":[],"taxon_id":1,"taxon_name":"species1","taxon_rank":"species","superkingdom_id":null,"kingdom_id":2,"subkingdom_id":null,"superphylum_id":null,"phylum_id":null,"subphylum_id":null,"superclass_id":null,"class_id":null,"subclass_id":null,"infraclass_id":null,"superorder_id":null,"order_id":null,"suborder_id":null,"infraorder_id":null,"parvorder_id":null,"superfamily_id":null,"family_id":null,"subfamily_id":null,"tribe_id":null,"subtribe_id":null,"genus_id":null,"subgenus_id":null,"species_group_id":null,"species_subgroup_id":null,"species_id":1,"subspecies_id":null,"varietas_id":null,"forma_id":null}
    ]'

    get :peptinfo, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true', extra: 'true', domains: 'true' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
