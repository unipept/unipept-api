# Example: cap dev deploy server=patty.taild1497.ts.net

set :stage, :dev

set :deploy_to, '/home/unipept/rails'

set :server, ENV['server'] || 'rick.taild1497.ts.net'

# don't specify db as it's not needed for unipept
server "#{fetch(:server)}", user: 'unipept', roles: %i[web app], ssh_options: {
  port: 4840
}

set :branch, 'feature/suffix-array-integration'
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
