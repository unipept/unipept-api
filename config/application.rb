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
      unipept: '4.6.4',
      gem: '2.2.1',
      uniprot: '2021.03',
      desktop: '1.2.4'
    }
    
    config.api_only = true

    config.api_host = 'api.unipept.ugent.be'
    
    MultiJson.use :Oj
  end
end
