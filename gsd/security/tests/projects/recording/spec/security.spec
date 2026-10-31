trust_boundary TB1 "Browser to API" {
  plan          v1.0/01/01-01
  boundary      "Browser to API"
  description   "Untrusted task titles reach the store"
  data_crossing "task titles (user input)"
}

threat T-01-01 "Injected title" {
  plan        v1.0/01/01-01
  category    tampering
  component   "src/tasks.js"
  severity    high
  disposition mitigate
  mitigation  "Titles are stored as data and never evaluated"
  files       ["src/tasks.js"]
}

threat T-01-02 "Unbounded titles" {
  plan        v1.0/01/01-01
  category    denial_of_service
  component   "src/tasks.js"
  severity    medium
  disposition mitigate
  mitigation  "Titles are capped"
}
