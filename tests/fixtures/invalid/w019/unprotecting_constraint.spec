spec "test" {
  version "1.0"
  plugins ["@specforge/governance"]
}

constraint response_time "Response Time" {
  category performance
  description """API responses MUST complete within 200ms"""
}
