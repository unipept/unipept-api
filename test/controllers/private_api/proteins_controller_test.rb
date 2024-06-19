require 'test_helper'

class PrivateApi::ProteinsControllerTest < ActionController::TestCase
  teardown :assert_success

  test 'should get proteins' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '{
      "lca":1,"common_lineage":[],"proteins":[{"uniprotAccessionId":"nr3","name":"some name","organism":2,"ecNumbers":[],"goTerms":[],"interproEntries":[]},{"uniprotAccessionId":"nr4","name":"some name","organism":1,"ecNumbers":[],"goTerms":[],"interproEntries":[]}]
    }'

    get :proteins, params: { peptide: "AAILER" }
  end

  test 'should get proteins with il' do
    stub_http_request! 'test/fixtures/index/response_equate.json'

    @expected = '{
      "lca":1, "common_lineage":[], "proteins":[{"uniprotAccessionId":"nr3", "name":"some name", "organism":2, "ecNumbers":[], "goTerms":[], "interproEntries":[]}, {"uniprotAccessionId":"nr4", "name":"some name", "organism":1, "ecNumbers":[], "goTerms":[], "interproEntries":[]}]
    }'

    get :proteins, params: { peptide: "AAILER", equate_il: 'true' }
  end

  test 'should get proteins no match' do
    stub_http_request! 'test/fixtures/index/response_empty.json'

    @expected = '{"lca":-1,"common_lineage":[],"proteins":[]}'

    get :proteins, params: { peptide: "AAAAAAAAA" }
  end

  test 'should get proteins too short sequence' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '{
      "name":"Sequence too short",
      "message":"The peptide sequence you provided is too short. It should contain at least 5 valid amino acids."
    }'

    get :proteins, params: { peptide: "AAA" }
  end

  test 'should get proteins without peptides' do
    stub_http_request! 'test/fixtures/index/response.json'

    @expected = '{
      "name":"Invalid peptide provided",
      "message":"No peptide sequence was provided. Please provide a valid peptide sequence."
    }'

    get :proteins, params: {}
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
