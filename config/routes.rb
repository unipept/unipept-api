Rails.application.routes.draw do
  # General information
  scope :private_api, as: 'private_api' do
    match "/*path" => "handle_options#handle_options_request", via: [:options]
    match "goterms" => "private_api/goterms#goterms", via: [:get, :post]
    match "ecnumbers" => "private_api/ecnumbers#ecnumbers", via: [:get, :post]
    match "taxa" => "private_api/taxa#taxa", via: [:get, :post]
    match "interpros" => "private_api/interpros#interpros", via: [:get, :post]
    match "proteins" => "private_api/proteins#proteins", via: [:get, :post]
    match "metadata" => "private_api/metadata#metadata", via: [:get, :post]
  end

  scope :mpa, as: 'mpa' do
    match '/*path' => 'handle_options#handle_options_request', via: [:options]  
    match 'pept2data' => 'mpa/pept2data#pept2data', via: %i[get post]
    match 'pept2filtered' => 'mpa/pept2filtered#pept2filtered', via: %i[get post]
  end

  scope :datasets, as: 'datasets' do
    match 'sampledata', via: [:post], to: 'datasets/sampledata#sampledata'
  end

  scope :api, as: 'api' do
    match '/*path', via: [:options], to: 'handle_options#handle_options_request'
  end

  namespace :api, path: 'api/v1' do
    match 'pept2taxa' => "pept2taxa#pept2taxa", via: %i[get post]
    match 'pept2lca' => "pept2lca#pept2lca", via: %i[get post]
    match 'taxa2lca' => 'taxa2lca#taxa2lca', via: %i[get post]
    match 'pept2prot' => 'pept2prot#pept2prot', via: %i[get post]
    match 'pept2funct' => 'pept2funct#pept2funct', via: %i[get post]
    match 'pept2ec' => 'pept2ec#pept2ec', via: %i[get post]
    match 'pept2go' => 'pept2go#pept2go', via: %i[get post]
    match 'pept2interpro' => 'pept2interpro#pept2interpro', via: %i[get post]
    match 'taxa2tree' => 'taxa2tree#taxa2tree', via: %i[get post]
    match 'peptinfo' => 'peptinfo#peptinfo', via: %i[get post]
    match 'taxonomy' => 'taxonomy#taxonomy', via: %i[get post]
    match 'messages' => 'api#messages', via: %i[get post]
    match 'protinfo' => 'protinfo#protinfo', via: %i[get post]
  end

  namespace :api, path: 'api/v2' do
    match 'pept2taxa' => "pept2taxa#pept2taxa", via: %i[get post]
    match 'pept2lca' => "pept2lca#pept2lca", via: %i[get post]
    match 'taxa2lca' => 'taxa2lca#taxa2lca', via: %i[get post]
    match 'pept2prot' => 'pept2prot#pept2prot', via: %i[get post]
    match 'pept2funct' => 'pept2funct#pept2funct', via: %i[get post]
    match 'pept2ec' => 'pept2ec#pept2ec', via: %i[get post]
    match 'pept2go' => 'pept2go#pept2go', via: %i[get post]
    match 'pept2interpro' => 'pept2interpro#pept2interpro', via: %i[get post]
    match 'pept2gm' => 'api#pept2gm', via: %i[get post]
    match 'taxa2tree' => 'taxa2tree#taxa2tree', via: %i[get post]
    match 'peptinfo' => 'peptinfo#peptinfo', via: %i[get post]
    match 'taxonomy' => 'taxonomy#taxonomy', via: %i[get post]
    match 'messages' => 'api#messages', via: %i[get post]
    match 'protinfo' => 'protinfo#protinfo', via: %i[get post]
  end
end
