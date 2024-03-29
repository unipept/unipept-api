class Api::ApiController < HandleOptionsController
  respond_to :json

  # before_action :log, only: %i[pept2taxa pept2lca pept2prot pept2funct pept2ec pept2go pept2interpro peptinfo taxa2lca taxonomy taxa2tree]

  # sends a message to the ruby cli
  def messages
    version = params[:version]
    gem_version = Rails.application.config.versions[:gem]
    if Gem::Version.new(gem_version) > Gem::Version.new(version)
      render plain: "Unipept gem #{gem_version} is released!. Run 'gem update unipept' to update."
    else
      render plain: ''
    end
  end

  private

  # log all api calls to stathat
  # def log
  #   return unless Rails.application.config.unipept_API_logging
  #
  #   StatHat::API.ez_post_count("API - #{action_name}", Rails.application.config.unipept_stathat_key, 1)
  # end

  # enable cross origin requests
  def set_headers
    headers['Access-Control-Allow-Origin'] = '*'
    headers['Access-Control-Expose-Headers'] = 'ETag'
    headers['Access-Control-Allow-Methods'] = 'GET, POST'
    headers['Access-Control-Allow-Headers'] = '*,x-requested-with,Content-Type,If-Modified-Since,If-None-Match'
    headers['Access-Control-Max-Age'] = '86400'
  end

  # handles the parameters
  def set_params
    # is the user using v1 of the API or v2?
    @v1 = request.env['PATH_INFO'].include? 'v1'
    unsafe_hash = params.to_unsafe_h
    @input = unsafe_hash[:input]
    case @input
    when Hash # hash
      @input = @input.values
    when String # string
      @input = if @input[0] == '[' # parse json
                 JSON.parse @input
               else # comma separated
                 @input.split(',')
               end
    end
    @input = [] if @input.nil?
    @input = @input.compact.map(&:chomp)
    @input_order = @input.dup

    @counts = unsafe_hash[:counts]
    @link = params[:link] == 'true'

    @equate_il = params[:equate_il] == 'true'
    @names = params[:names] == 'true'
    @domains = params[:domains] == 'true'
    @extra_info = params[:extra] == 'true'
    @remove = params[:remove] == 'true'

    @input = @input.map { |s| s.tr('I', 'L') } if @equate_il
  end

  # prepares the taxonomy query
  def set_query
    @query = if @extra_info
               if @names
                 Taxon.includes(lineage: Lineage::ORDER_T)
               else
                 Taxon.includes(:lineage)
               end
             else
               Taxon
             end
  end

  # prepares the sequences query
  def set_sequences
    rel_name = @equate_il ? :peptides : :original_peptides
    @sequences = Sequence.joins(rel_name => :uniprot_entry)
                         .where(sequence: @input)
  end

  # Reorders the results according to the input order
  def filter_input_order
    @input_order.select! do |s|
      key = @equate_il ? s.tr('I', 'L') : s
      @result.key? key
    end
  end
end
