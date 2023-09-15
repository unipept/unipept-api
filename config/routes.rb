Rails.application.routes.draw do
  # General information
  scope :private_api, as: 'private_api' do
    match "/*path", via: [:options], :to => "handle_options#handle_options_request"
    match "goterms", via: [:get, :post], :to => "private_api#goterms"
    match "ecnumbers",     via: [:get, :post], :to => "private_api#ecnumbers"
    match "taxa",     via: [:get, :post], :to => "private_api#taxa"
    match "interpros", via: [:get, :post], :to => "private_api#interpros"
    match "proteins", via: [:get, :post], :to => "private_api#proteins"
    match "metadata", via: [:get, :post], :to => "private_api#metadata"
  end

  scope :mpa, as: 'mpa' do
    match '/*path', via: [:options], to: 'handle_options#handle_options_request'
    match 'pept2data', via: %i[get post], to: 'mpa#pept2data'
    match 'pept2filtered', via: %i[get post], to: 'mpa#pept2filtered'
  end

  scope :datasets, as: 'datasets' do
    match 'sampledata', via: [:post], to: 'datasets#sampledata'
  end

  scope :api, as: 'api' do
    match '/*path', via: [:options], to: 'handle_options#handle_options_request'
  end

  namespace :api, path: 'api/v1' do
    match 'pept2taxa' => "api#pept2taxa", via: %i[get post]
    match 'pept2lca' => "api#pept2lca", via: %i[get post]
    match 'taxa2lca' => 'api#taxa2lca', via: %i[get post]
    match 'pept2prot' => 'api#pept2prot', via: %i[get post]
    match 'pept2funct' => 'api#pept2funct', via: %i[get post]
    match 'pept2ec' => 'api#pept2ec', via: %i[get post]
    match 'pept2go' => 'api#pept2go', via: %i[get post]
    match 'pept2interpro' => 'api#pept2interpro', via: %i[get post]
    match 'taxa2tree' => 'api#taxa2tree', via: %i[get post]
    match 'peptinfo' => 'api#peptinfo', via: %i[get post]
    match 'taxonomy' => 'api#taxonomy', via: %i[get post]
    match 'messages' => 'api#messages', via: %i[get post]
    match 'protinfo' => 'api#protinfo', via: %i[get post]
  end

  namespace :api, path: 'api/v2' do
    match '/*path', via: [:options], to: 'handle_options#handle_options_request'
    match 'pept2taxa' => "api#pept2taxa", via: %i[get post]
    match 'pept2lca' => "api#pept2lca", via: %i[get post]
    match 'taxa2lca' => 'api#taxa2lca', via: %i[get post]
    match 'pept2prot' => 'api#pept2prot', via: %i[get post]
    match 'pept2funct' => 'api#pept2funct', via: %i[get post]
    match 'pept2ec' => 'api#pept2ec', via: %i[get post]
    match 'pept2go' => 'api#pept2go', via: %i[get post]
    match 'pept2interpro' => 'api#pept2interpro', via: %i[get post]
    match 'pept2gm' => 'api#pept2gm', via: %i[get post]
    match 'taxa2tree' => 'api#taxa2tree', via: %i[get post]
    match 'peptinfo' => 'api#peptinfo', via: %i[get post]
    match 'taxonomy' => 'api#taxonomy', via: %i[get post]
    match 'messages' => 'api#messages', via: %i[get post]
    match 'protinfo' => 'api#protinfo', via: %i[get post]
  end
end
