module SuffixArrayHelper
  def search(peptides, equate_il)
    if peptides.empty?
      return
    end

    # Convert the peptides array into a json string
    json_data = { peptides: peptides }.to_json

    # The URL to the suffix array search service
    url = URI.parse("http://localhost:3000/search")

    # Create a new HTTP POST request
    request = Net::HTTP::Post.new(url.path)
    request.content_type = "application/json"
    request.body = json_data

    # Send the request to the suffix array search service
    response = Net::HTTP.start(url.host, url.port) do |http|
      http.request(request)
    end

    # Parse the response from the suffix array search service
    JSON.parse(response.body)
  end
end
