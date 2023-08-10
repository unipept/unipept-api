set :stage, :dev

set :deploy_to, '/home/unipept/rails'

set :server, ENV['server'] || 'rick.ugent.be'

#set :rvm_custom_path, '/usr/share/rvm'

# don't specify db as it's not needed for unipept
server "#{fetch(:server)}", user: 'unipept', roles: %i[web app], ssh_options: {
  port: 4840
}

set :branch, 'feature/pept2filtered-cutoff'
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

