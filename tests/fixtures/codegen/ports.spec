spec "codegen-ports" {
  version "1.0"
  plugins []
}

type User {
  name string
  email string
}

port UserRepository {
  method save(user: User) -> Result<void, Error>
  method findById(id: string) -> User?
  method list() -> User[]
  method delete(id: string) -> Result<void, Error>
}

port FileSystem {
  method read(path: string) -> Result<string, Error>
  method write(path: string, content: string) -> Result<void, Error>
  method exists(path: string) -> boolean
}
