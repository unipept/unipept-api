require 'test_helper'

class Mpa::Pept2filteredControllerTest < ActionController::TestCase
  teardown :assert_success

  # TODO: find a working (non-empty) example
  # test 'should get pept2filtered' do
  #   @expected = '{}'

  #   get :pept2filtered, params: { peptides: %w[AAIER AAILER], taxa: %w[1 2 13 15], format: 'json' }
  # end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
