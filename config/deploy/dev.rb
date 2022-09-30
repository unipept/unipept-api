set :stage, :dev

set :deploy_to, '/home/unipept/rails'

# don't specify db as it's not needed for unipept
server 'sherlock.ugent.be', user: 'unipept', roles: %i[web app], ssh_options: {
  port: 4840
}

set :branch, 'develop'
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
