ENV["RAILS_ENV"] ||= "test"
require_relative "../config/environment"
require "rails/test_help"

require 'webmock'
include WebMock::API
WebMock.enable!

require 'simplecov'
SimpleCov.start 'rails'

require 'simplecov-cobertura'
SimpleCov.formatter = SimpleCov::Formatter::CoberturaFormatter

def stub_http_request!(response_json_file)
  response = File.new Rails.root.join(response_json_file)

  stub_request(:post, 'http://localhost:3000/analyse')
    .to_return(body: response, headers: { 'Content-Type' => 'application/json' }, status: 200)
end

class ActiveSupport::TestCase
  # Run tests in parallel with specified workers
  parallelize(workers: :number_of_processors)

  parallelize_setup do |worker|
    SimpleCov.command_name "#{SimpleCov.command_name}-#{worker}"
  end

  parallelize_teardown do |_worker|
    SimpleCov.result
  end

  # Setup all fixtures in test/fixtures/*.yml for all tests in alphabetical order.
  fixtures :all

  def assert_json
    assert_equal JSON.parse(@response.body), JSON.parse(@expected)
  end
end
