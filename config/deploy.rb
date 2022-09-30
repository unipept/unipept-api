set :application, 'unipept-api'
set :repo_url,  'https://github.com/unipept/unipept-api.git'

# set :linked_files, %w{config/database.yml}
append :linked_dirs, '.bundle'
append :linked_dirs, 'log'
append :linked_dirs, 'tmp'
append :linked_dirs, 'vendor/bundle'
append :linked_dirs, 'public/system'

namespace :deploy do
  desc 'Restart application'
  task :restart do
    on roles(:web) do
      execute :touch, release_path.join('tmp', 'restart.txt')
    end
  end

  after :restart, :clear_cache do
    on roles(:web), in: :groups, limit: 3, wait: 10 do
      # Here we can do anything such as:
      # within release_path do
      #   execute :rake, 'cache:clear'
      # end
    end
  end

  after :finishing, 'deploy:cleanup'
  after 'deploy:publishing', 'deploy:restart'
end
