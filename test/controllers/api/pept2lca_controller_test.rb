require 'test_helper'

class Api::Pept2lcaControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get pept2lca' do
    @expected = '[
      {"peptide":"AAIER","taxon_id":2,"taxon_name":"kingdom1","taxon_rank":"kingdom"},
      {"peptide":"AAILER","taxon_id":1,"taxon_name":"species1","taxon_rank":"species"}
    ]'
    
    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json' }
  end

  test 'should get pept2lca with il' do
    @expected = '[
      {"peptide":"AAIER","taxon_id":1,"taxon_name":"species1","taxon_rank":"species"},
      {"peptide":"AAILER","taxon_id":1,"taxon_name":"species1","taxon_rank":"species"}
    ]'

    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true' }
  end

  test 'should get pept2lca with extra' do
    @expected = '[
      {"peptide":"AAIER","taxon_id":2,"taxon_name":"kingdom1","taxon_rank":"kingdom","superkingdom_id":null,"kingdom_id":2,"subkingdom_id":null,"superphylum_id":null,"phylum_id":null,"subphylum_id":null,"superclass_id":null,"class_id":null,"subclass_id":null,"infraclass_id":null,"superorder_id":null,"order_id":null,"suborder_id":null,"infraorder_id":null,"parvorder_id":null,"superfamily_id":null,"family_id":null,"subfamily_id":null,"tribe_id":null,"subtribe_id":null,"genus_id":null,"subgenus_id":null,"species_group_id":null,"species_subgroup_id":null,"species_id":null,"subspecies_id":null,"varietas_id":null,"forma_id":null},
      {"peptide":"AAILER","taxon_id":1,"taxon_name":"species1","taxon_rank":"species","superkingdom_id":null,"kingdom_id":2,"subkingdom_id":null,"superphylum_id":null,"phylum_id":null,"subphylum_id":null,"superclass_id":null,"class_id":null,"subclass_id":null,"infraclass_id":null,"superorder_id":null,"order_id":null,"suborder_id":null,"infraorder_id":null,"parvorder_id":null,"superfamily_id":null,"family_id":null,"subfamily_id":null,"tribe_id":null,"subtribe_id":null,"genus_id":null,"subgenus_id":null,"species_group_id":null,"species_subgroup_id":null,"species_id":1,"subspecies_id":null,"varietas_id":null,"forma_id":null}
    ]'

    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json', extra: 'true' }
  end

  test 'should get pept2lca with names' do
    @expected = '[
      {"peptide":"AAIER","taxon_id":2,"taxon_name":"kingdom1","taxon_rank":"kingdom"},
      {"peptide":"AAILER","taxon_id":1,"taxon_name":"species1","taxon_rank":"species"}
    ]'

    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json', names: 'true' }
  end

  test 'should get pept2lca with extra and names' do
    @expected = '[
      {"peptide":"AAIER","taxon_id":2,"taxon_name":"kingdom1","taxon_rank":"kingdom","superkingdom_id":null,"superkingdom_name":"","kingdom_id":2,"kingdom_name":"kingdom1","subkingdom_id":null,"subkingdom_name":"","superphylum_id":null,"superphylum_name":"","phylum_id":null,"phylum_name":"","subphylum_id":null,"subphylum_name":"","superclass_id":null,"superclass_name":"","class_id":null,"class_name":"","subclass_id":null,"subclass_name":"","infraclass_id":null,"infraclass_name":"","superorder_id":null,"superorder_name":"","order_id":null,"order_name":"","suborder_id":null,"suborder_name":"","infraorder_id":null,"infraorder_name":"","parvorder_id":null,"parvorder_name":"","superfamily_id":null,"superfamily_name":"","family_id":null,"family_name":"","subfamily_id":null,"subfamily_name":"","tribe_id":null,"tribe_name":"","subtribe_id":null,"subtribe_name":"","genus_id":null,"genus_name":"","subgenus_id":null,"subgenus_name":"","species_group_id":null,"species_group_name":"","species_subgroup_id":null,"species_subgroup_name":"","species_id":null,"species_name":"","subspecies_id":null,"subspecies_name":"","varietas_id":null,"varietas_name":"","forma_id":null,"forma_name":""},
      {"peptide":"AAILER","taxon_id":1,"taxon_name":"species1","taxon_rank":"species","superkingdom_id":null,"superkingdom_name":"","kingdom_id":2,"kingdom_name":"kingdom1","subkingdom_id":null,"subkingdom_name":"","superphylum_id":null,"superphylum_name":"","phylum_id":null,"phylum_name":"","subphylum_id":null,"subphylum_name":"","superclass_id":null,"superclass_name":"","class_id":null,"class_name":"","subclass_id":null,"subclass_name":"","infraclass_id":null,"infraclass_name":"","superorder_id":null,"superorder_name":"","order_id":null,"order_name":"","suborder_id":null,"suborder_name":"","infraorder_id":null,"infraorder_name":"","parvorder_id":null,"parvorder_name":"","superfamily_id":null,"superfamily_name":"","family_id":null,"family_name":"","subfamily_id":null,"subfamily_name":"","tribe_id":null,"tribe_name":"","subtribe_id":null,"subtribe_name":"","genus_id":null,"genus_name":"","subgenus_id":null,"subgenus_name":"","species_group_id":null,"species_group_name":"","species_subgroup_id":null,"species_subgroup_name":"","species_id":1,"species_name":"species1","subspecies_id":null,"subspecies_name":"","varietas_id":null,"varietas_name":"","forma_id":null,"forma_name":""}
    ]'

    get :pept2lca, params: { input: %w[AAIER AAILER], extra: 'true', names: 'true' }, format: 'json'
  end

  test 'should get pept2lca with extra and names and il' do
    @expected = '[
      {"peptide":"AAIER","taxon_id":1,"taxon_name":"species1","taxon_rank":"species","superkingdom_id":null,"superkingdom_name":"","kingdom_id":2,"kingdom_name":"kingdom1","subkingdom_id":null,"subkingdom_name":"","superphylum_id":null,"superphylum_name":"","phylum_id":null,"phylum_name":"","subphylum_id":null,"subphylum_name":"","superclass_id":null,"superclass_name":"","class_id":null,"class_name":"","subclass_id":null,"subclass_name":"","infraclass_id":null,"infraclass_name":"","superorder_id":null,"superorder_name":"","order_id":null,"order_name":"","suborder_id":null,"suborder_name":"","infraorder_id":null,"infraorder_name":"","parvorder_id":null,"parvorder_name":"","superfamily_id":null,"superfamily_name":"","family_id":null,"family_name":"","subfamily_id":null,"subfamily_name":"","tribe_id":null,"tribe_name":"","subtribe_id":null,"subtribe_name":"","genus_id":null,"genus_name":"","subgenus_id":null,"subgenus_name":"","species_group_id":null,"species_group_name":"","species_subgroup_id":null,"species_subgroup_name":"","species_id":1,"species_name":"species1","subspecies_id":null,"subspecies_name":"","varietas_id":null,"varietas_name":"","forma_id":null,"forma_name":""},
      {"peptide":"AAILER","taxon_id":1,"taxon_name":"species1","taxon_rank":"species","superkingdom_id":null,"superkingdom_name":"","kingdom_id":2,"kingdom_name":"kingdom1","subkingdom_id":null,"subkingdom_name":"","superphylum_id":null,"superphylum_name":"","phylum_id":null,"phylum_name":"","subphylum_id":null,"subphylum_name":"","superclass_id":null,"superclass_name":"","class_id":null,"class_name":"","subclass_id":null,"subclass_name":"","infraclass_id":null,"infraclass_name":"","superorder_id":null,"superorder_name":"","order_id":null,"order_name":"","suborder_id":null,"suborder_name":"","infraorder_id":null,"infraorder_name":"","parvorder_id":null,"parvorder_name":"","superfamily_id":null,"superfamily_name":"","family_id":null,"family_name":"","subfamily_id":null,"subfamily_name":"","tribe_id":null,"tribe_name":"","subtribe_id":null,"subtribe_name":"","genus_id":null,"genus_name":"","subgenus_id":null,"subgenus_name":"","species_group_id":null,"species_group_name":"","species_subgroup_id":null,"species_subgroup_name":"","species_id":1,"species_name":"species1","subspecies_id":null,"subspecies_name":"","varietas_id":null,"varietas_name":"","forma_id":null,"forma_name":""}
    ]'

    get :pept2lca, params: { input: %w[AAIER AAILER], format: 'json', equate_il: 'true', extra: 'true', names: 'true' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
