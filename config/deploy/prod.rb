set :stage, :prod

set :deploy_to, '/home/unipept/rails'

# don't specify db as it's not needed for unipept
server 'rick.ugent.be', user: 'unipept', roles: %i[web app], ssh_options: {
  port: 4840
}

set :branch, 'main'
set :rails_env, :production
