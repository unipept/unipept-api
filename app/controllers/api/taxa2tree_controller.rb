require 'octokit'

class Api::Taxa2treeController < Api::ApiController
  before_action :set_headers
  before_action :set_params

  # Returns a tree with all taxa aggregated over the complete lineage.
  # param[input]: Array, required, List of input taxon ids
  # param[extra]: "true" or "false", Include lineage
  # param[names]: "true" or "false", Include the lineage names
  def taxa2tree
    frequencies = Hash.new 0
    if @counts
      # Convert @counts into a hash with default values and integer keys.
      @counts.each do |k, v|
        frequencies[k.to_i] = v.to_i
      end
    else
      @input.each do |id|
        frequencies[id.to_i] += 1
      end
    end

    @root = Lineage.build_tree(frequencies)

    if @link
      client = Octokit::Client.new(access_token: ENV['TAXA2TREE_AT'])
      result = client.create_gist(
        {
          description: 'Unipept Taxa2Tree results',
          files:
            {
              'index.html' => { content: render_to_string(template: 'api/taxa2tree/taxa2tree.html', layout: false) },
              'readme.md' => { content: render_to_string(template: 'api/taxa2tree_readme.md', layout: false) },
              '.block' => { content: 'height: 710' }
            },
          public: false
        }
      )

      @gist = result[:html_url]

      if @remove
        # Immediately delete the gist again. This is used for testing the uptime of the API server
        client.delete_gist(result[:id])
      end
    end

    render layout: false
  end
end
