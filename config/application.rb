require_relative "boot"

require "rails/all"
require 'multi_json'

# Require the gems listed in Gemfile, including any gems
# you've limited to :test, :development, or :production.
Bundler.require(*Rails.groups)

module UnipeptApi
  class Application < Rails::Application
    # Initialize configuration defaults for originally generated Rails version.
    config.load_defaults 7.0

    config.versions = {
      unipept: '5.0.9',
      gem: '3.0.2',
      uniprot: '2024.01',
      desktop: '2.0.0'
    }

    config.api_only = true

    config.api_host = 'api.unipept.ugent.be'

    MultiJson.use :Oj
  end
end
