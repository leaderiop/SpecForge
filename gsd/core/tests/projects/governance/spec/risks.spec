risk RK1 "A key is written in clear" {
  cause        "put() skips encryption on an error path"
  effect       "a key lies readable on disk"
  severity     7
  occurrence   5
  detection    5
  post {
    occurrence 1
  }
  mitigated_by [v1.0/01/01-01/T1]
  mitigation   "encrypt before every write, and test the error path"
  affects      [GOV-01]
  tags         ["storage"]
}

risk RK2 "A rotation is slow" {
  cause      "every secret is re-encrypted at once"
  effect     "rotate() blocks for a few seconds"
  severity   2
  occurrence 3
  detection  1
}

risk RK3 "Keys rot" {
  cause      "a rotation stops halfway"
  effect     "part of the store is under a key nobody holds"
  severity   10
  occurrence 5
  detection  7
  milestone  v1.0
}

risk RK4 "A read returns another key" {
  cause        "the index and the file drift apart"
  effect       "a caller decrypts with the wrong key"
  severity     5
  occurrence   5
  detection    5
  post {
    severity  3
    detection 2
  }
  mitigated_by [v1.0/01/01-01/TR1]
}
