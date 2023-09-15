set :stage, :dev

set :deploy_to, '/home/unipept/rails'

set :server, ENV['server'] || 'patty.ugent.be'

# don't specify db as it's not needed for unipept
server "#{fetch(:server)}", user: 'unipept', roles: %i[web app], ssh_options: {
  port: 4840
}

set :branch, 'fix/options_request_api'
set :rails_env, :development

namespace :deploy do
  before :publishing, :block_robots do
    on roles :all do
      content = [
        '# This is a staging site. Do not index.',
        'User-agent: *',
        'Disallow: /'
      ].join($/)

      upload! StringIO.new(content), "#{release_path}/public/robots.txt"
    end
  end
end
