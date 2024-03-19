require 'test_helper'

class Datasets::SampledataControllerTest < ActionController::TestCase
  teardown :assert_success
  
  test 'should get sampledata' do
    @expected = '{
      "sample_data":[
        {"id":1,"environment":"env","reference":"ref","url":"url","project_website":"website","datasets":[{"name":"name","data":["data_data"],"order":null}]}
      ]
    }'

    post :sampledata, params: { format: 'json' }
  end

  private

  def assert_success
    assert_response :success
    assert_equal '*', @response.headers['Access-Control-Allow-Origin']
    assert_json
  end
end
