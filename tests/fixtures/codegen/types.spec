spec "codegen-types" {
  version "1.0"
  plugins []
}

type User {
  name string
  email string @unique
  age integer @optional
  id string @readonly
}

type Address {
  street string
  city string
  zip string
  country string
}

type UserStatus = active | inactive | banned

type ErrorCode = not_found | unauthorized | internal
