require 'test_helper'

class PrivateApi::ProteinsControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get proteins' do
    @expected = '{
      "lca":1,"common_lineage":[2,1],"proteins":[{"uniprotAccessionId":"nr2","name":"some name","organism":2,"ecNumbers":[],"goTerms":[],"interproEntries":[]},{"uniprotAccessionId":"nr","name":"some name","organism":1,"ecNumbers":["1.2.3.4"],"goTerms":["goid"],"interproEntries":["IPR000126"]}]
    }'

    get :proteins, params: { peptide: "AAIER" }
  end

  test 'should get proteins with il' do
    @expected = '{
      "lca":1,"common_lineage":[2,1],"proteins":[{"uniprotAccessionId":"nr2","name":"some name","organism":2,"ecNumbers":[],"goTerms":[],"interproEntries":[]},{"uniprotAccessionId":"nr","name":"some name","organism":1,"ecNumbers":["1.2.3.4"],"goTerms":["goid"],"interproEntries":["IPR000126"]}]
    }'

    get :proteins, params: { peptide: "AAIER", equate_il: 'true' }
  end

  test 'should get proteins no match' do
    @expected = '{"lca":-1,"common_lineage":[],"proteins":[]}'

    get :proteins, params: { peptide: "AAAAAAAAA" }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
