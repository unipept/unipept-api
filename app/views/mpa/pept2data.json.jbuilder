json.peptides(@response.values)
json.index_time @timings["index_time"]
json.index_parse_time @timings["index_parse_time"]
json.database_time @timings["database_time"]
json.aggregation_time @timings["aggregation_time"]
json.total_time @timings["total_time"]
