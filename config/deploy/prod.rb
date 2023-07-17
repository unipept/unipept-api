set :stage, :prod

set :deploy_to, '/home/unipept/rails'

set :server, ENV['server'] || 'patty.ugent.be'

set :rvm_custom_path, '/usr/share/rvm'

# don't specify db as it's not needed for unipept
server "#{fetch(:server)}", user: 'unipept', roles: %i[web app], ssh_options: {
  port: 4840
}

set :branch, 'main'
set :rails_env, :production
