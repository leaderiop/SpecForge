spec "gen-verify" {
  version "1.0"
  plugins []

  gen typescript {
    out "generated/ts"
    result "plain"
    naming "camelCase"
  }
}

type User {
  name string
  email string
}

port UserRepository {
  method save(user: User) -> Result<void, Error>
  method findById(id: string) -> User?
  method list() -> User[]
}

port FileSystem {
  method read(path: string) -> Result<string, Error>
  method write(path: string, content: string) -> Result<void, Error>
  method exists(path: string) -> boolean
}
