require 'test_helper'

class Api::ApiControllerTest < ActionController::TestCase
  test 'should get messages for old version' do
    get :messages, params: { version: '0' }
    assert_response :success
    assert @response.body.present?
  end

  test 'should not get message for current version' do
    get :messages, params: { version: Rails.application.config.versions[:gem] }
    assert_response :success
    assert @response.body.blank?
  end
end
