// Code generation types

type GeneratedFile {
  path       string
  content    string
  checksum   string   @readonly
}

type GenOutput {
  files      GeneratedFile[]
  language   string
  fromSpec   string   @readonly
}

type DriftResult {
  stale      boolean
  diffs      DriftDiff[]
}

type DriftDiff {
  path       string
  expected   string
  actual     string
}

type AdapterVerification {
  portName   string
  adapterPath string
  missing    string[]
  extra      string[]
  valid      boolean
}
