spec "gen-basic" {
  version "1.0"
  plugins []

  gen typescript {
    out "generated/ts"
    result "plain"
    readonly true
    naming "camelCase"
    tests "@specforge/vitest"
  }

  gen json-schema {
    out "generated/schemas"
  }
}

type User {
  name string
  email string @unique
  age integer @optional
  id string @readonly
}

type UserStatus = active | inactive | banned

type Address {
  street string
  city string
  zip string
}

port UserRepository {
  method save(user: User) -> Result<void, Error>
  method findById(id: string) -> User?
  method list() -> User[]
}

invariant data_integrity "Data Integrity" {
  guarantee """all user data MUST be validated"""
  enforced_by [validate_input]
  risk high
}

behavior validate_input "Validate Input" {
  invariants [data_integrity]
  contract """the system MUST validate all input fields"""
  verify unit "rejects empty email"
  verify unit "accepts valid email"
  verify integration "persists validated user"
}

feature user_management "User Management" {
  behaviors [validate_input]
  problem """user data can be invalid"""
  solution """validate all fields before persistence"""
  status active
}
