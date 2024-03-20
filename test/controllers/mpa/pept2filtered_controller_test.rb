require 'test_helper'

class Mpa::Pept2filteredControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get pept2filtered' do
    @expected = '{
      "peptides":[
        {"sequence":"AALER","taxa":[1,2],"fa":{"go_terms":["goid"],"ec_numbers":["EC:1.2.3.4"],"interpro_entries":["IPR:000126"]}},
        {"sequence":"AALLER","taxa":[2,1],"fa":{"go_terms":[],"ec_numbers":[],"interpro_entries":[]}}
      ]
    }'

    get :pept2filtered, params: { peptides: %w[AAIER AALER AAILER AALLER], taxa: %w[1 2 13 14 15], format: 'json' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
