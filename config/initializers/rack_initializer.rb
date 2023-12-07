if Rack::Utils.respond_to?("key_space_limit=")
  # Increase limit to 10 MiB
  Rack::Utils.key_space_limit = 10485760
end
