# RES-20: Type System Syntax Reference

**Companion Document to RES-20 Research**
**Version:** 1.0
**Date:** March 4, 2026

---

## Quick Reference Card

```spec
// REFINEMENT TYPES (v2.2)
type Money {
  amount   number { > 0 }                      // Numeric bound
  currency string { in ["USD", "EUR", "GBP"] } // Set membership
}

type User {
  email string { matches "^[a-z0-9]+@[a-z]+\\.[a-z]{2,}$" } // Regex
  age   integer { >= 18, <= 120 }                           // Range
}

// TYPED CONTRACTS (v2.2)
behavior create_user {
  contract {
    requires cmd.email != ""              // Precondition
    requires !exists(User, email: cmd.email)

    ensures result.id != ""               // Postcondition
    ensures result.email == cmd.email
    ensures produced(user_created)        // Side effect
  }
}

// EVENT PAYLOAD COMPATIBILITY (v2.3)
event user_created {
  payload {
    userId string { len > 0 }
    email  string
  }
  consumers [send_notification]
}

behavior send_notification {
  consumes user_created { userId, email }  // Payload projection
}

// CONTRACT SUBTYPING (v2.4)
behavior base_operation {
  contract {
    requires input != ""
    ensures result != ""
  }
}

behavior enhanced_operation extends base_operation {
  contract {
    requires input != ""            // SAME precondition (OK)
    ensures result != ""
    ensures result.length > 10      // STRONGER postcondition (OK)
  }
}

// PORT SUBTYPING (v2.5)
port BaseRepository {
  method findById(id: string) -> Result<Entity, NotFoundError>
}

port UserRepository extends BaseRepository {
  method findById(id: string) -> Result<User, NotFoundError>  // Covariant return
}

// SET CONSTRAINTS (v2.6)
port UserRepository {
  method findByRole(role: UserRole) -> Result<User[], never>
    where result ⊆ {u ∈ User | u.role == role}  // Subset constraint
}

type User {
  role UserRole

  invariant "role ∈ {admin, editor, viewer}"  // Set membership
}
```

---

## 1. Refinement Type Syntax

### 1.1 Numeric Constraints

```spec
type Product {
  quantity  integer { > 0 }                   // Positive integer
  price     number { >= 0.01 }                // Minimum value
  discount  number { >= 0.0, <= 1.0 }         // Percentage (range)
  stock     integer { >= 0, <= 10000 }        // Bounded inventory
}
```

**Operators:** `>`, `>=`, `<`, `<=`, `==`, `!=`

**Type compatibility:**
- `number`: all numeric operators
- `integer`: all numeric operators
- `string`: only `==`, `!=` (for exact match)
- `boolean`: only `==`, `!=`
- `timestamp`: `<`, `<=`, `>`, `>=` (temporal ordering)

### 1.2 String Constraints

```spec
type User {
  email    string { matches "^[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}$" }
  username string { len >= 3, len <= 20 }
  role     string { in ["admin", "editor", "viewer"] }
  apiKey   string { len == 32 }
}
```

**Functions:**
- `len` — string length
- `matches` — regex pattern (PCRE syntax, backslash-escaped)
- `in [...]` — enum set membership

### 1.3 Timestamp Constraints

```spec
type Event {
  createdAt timestamp { <= now() }            // Can't be in future
  expiresAt timestamp { > createdAt }         // Must be after creation
  updatedAt timestamp { >= createdAt }        // Must be after or equal
}
```

**Functions:**
- `now()` — current timestamp (evaluation time)
- Operators: `<`, `<=`, `>`, `>=`, `==`, `!=`

### 1.4 Derived Field Constraints

```spec
type OrderItem {
  quantity  integer { > 0 }
  unitPrice number { > 0 }
  total     number { == quantity * unitPrice }  // Derived constraint
}

type Rectangle {
  width  number { > 0 }
  height number { > 0 }
  area   number { == width * height }
}
```

**Expressions:**
- Arithmetic: `+`, `-`, `*`, `/`
- Field references: `quantity`, `unitPrice`
- Parentheses: `(width + height) * 2`

**Limitation:** Derived constraints are **documentation only** in v2.2. Compiler validates syntax but doesn't enforce equality. Full enforcement requires SMT solver (v3.0).

### 1.5 Collection Constraints

```spec
type Order {
  items OrderItem[] { |items| > 0, |items| <= 100 }  // Non-empty, bounded
}

type User {
  roles string[] { |roles| >= 1 }  // At least one role
}
```

**Operators:**
- `|collection|` — cardinality (size)
- Collection refinements apply to the collection itself, not elements
- For element constraints, use type-level invariants (see section 2.1)

---

## 2. Type-Level Invariants

### 2.1 Type Invariant Syntax

```spec
type User {
  id        string      @readonly
  email     string      @unique
  role      UserRole
  createdAt timestamp   @readonly

  invariant "email matches ^[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}$"
  invariant "role ∈ {admin, editor, viewer}"
  invariant "createdAt <= now()"
}
```

**Syntax:**
```ebnf
type_invariant = "invariant" , string_literal ;
```

Type invariants are **prose + optional formalization**. The string can be:
- Human-readable English: `"email must be valid"`
- Formal predicate: `"email matches regex"`
- Set-theoretic: `"role ∈ {admin, editor, viewer}"`

**Difference from field refinements:**

| Field Refinement | Type Invariant |
|------------------|----------------|
| Per-field constraint | Entire type constraint |
| `email string { matches "..." }` | `invariant "email matches ..."` |
| Enforced on field writes | Enforced on type instantiation |
| Syntax validated, checked statically | Prose/formal hybrid, checked heuristically |

**Use field refinements when possible** (stronger static checking). Use type invariants for:
- Cross-field constraints: `invariant "expiresAt > createdAt"`
- Collection element constraints: `invariant "all items have positive quantity"`
- Complex predicates: `invariant "if role == admin then permissions.length > 0"`

### 2.2 Cross-Field Invariants

```spec
type DateRange {
  startDate timestamp
  endDate   timestamp

  invariant "endDate >= startDate"
}

type Money {
  amount   number
  currency string

  invariant "if amount > 0 then currency in {USD, EUR, GBP}"
}

type Order {
  items      OrderItem[]
  totalAmount number
  currency   string

  invariant "totalAmount == sum(items.map(i => i.total.amount))"
  invariant "all items have currency == order.currency"
}
```

**Limitations (v2.2-2.6):**
- Cross-field invariants are **validated heuristically** (pattern matching)
- No SMT proof in v2.x — requires v3.0 `--verify` flag
- Compiler warns if it can't validate: W059 "unknown invariant predicate"

---

## 3. Typed Contract Syntax

### 3.1 Requires/Ensures Clauses

```spec
behavior create_user {
  contract {
    requires cmd.email != ""
    requires cmd.email matches "^[a-z0-9]+@[a-z]+\\.[a-z]{2,}$"
    requires !exists(User, email: cmd.email)

    ensures result.id != ""
    ensures result.email == cmd.email
    ensures result.createdAt <= now()
    ensures produced(user_created)
  }
}
```

**Syntax:**
```ebnf
contract      = "contract" , "{" , { contract_clause } , "}" ;
contract_clause = requires_clause | ensures_clause ;
requires_clause = "requires" , predicate ;
ensures_clause  = "ensures" , predicate ;
predicate     = expression ;
```

**Built-in predicates:**
- `exists(Type, field: value)` — entity exists with matching field
- `produced(event_id)` — event was emitted
- `sent(message_id)` — message was sent (for async systems)

### 3.2 Contract + Field Refinement Interaction

```spec
type PositiveMoney {
  amount   number { > 0 }
  currency string
}

behavior transfer_funds {
  contract {
    // This is REDUNDANT (refinement already ensures amount > 0)
    requires amount.amount > 0    // Compiler emits I008: "redundant with refinement"

    // This is REQUIRED (not enforced by refinement)
    requires from.balance >= amount.amount

    ensures from.balance' == from.balance - amount.amount
    ensures to.balance' == to.balance + amount.amount
  }
}
```

**Compiler optimization:**
- Compiler prunes redundant `requires` clauses that are implied by refinements
- Warning: W023 "contract clause redundant with refinement type"

### 3.3 Primed Variables (State Change)

```spec
behavior update_balance {
  contract {
    requires account.balance >= amount
    ensures account.balance' == account.balance - amount  // ' denotes "after" state
  }
}
```

**Syntax:** `field'` denotes **post-state** (value after operation).

**Semantics:**
- `field` — pre-state (value before operation)
- `field'` — post-state (value after operation)

Used for expressing state transitions in contracts.

---

## 4. Event Payload Typing

### 4.1 Payload Definition

```spec
event user_created {
  trigger   create_user
  channel   "users.created"

  payload {
    userId    string { len > 0 }     // Refinement on payload field
    email     string
    role      UserRole
    timestamp timestamp
  }

  consumers [send_notification, log_audit]
}
```

**Payload fields follow same syntax as type fields:**
- Primitive types: `string`, `number`, `integer`, `boolean`, `timestamp`
- Named types: `UserRole`, `Money`, etc.
- Refinements: `{ predicate }`
- Annotations: `@readonly`, `@optional`

### 4.2 Consumer Payload Projection

```spec
behavior send_notification {
  // Full payload consumption (implicit)
  consumes [user_created]

  contract {
    requires event.userId != ""
    requires event.email != ""
    ensures notification_sent(event.userId)
  }
}

behavior log_audit {
  // Partial payload consumption (explicit projection)
  consumes user_created { userId, timestamp }

  contract {
    requires event.userId != ""
    ensures audit_logged(event.userId, event.timestamp)
  }
}
```

**Syntax:**
```ebnf
consumes_clause = "consumes" , event_ref , [ payload_projection ] ;
payload_projection = "{" , identifier , { "," , identifier } , "}" ;
```

**Type Checking Rules:**
1. All fields in projection MUST exist in event payload (else E041)
2. Consumer can use only fields in projection (else E039)
3. Projection enables payload evolution (adding optional fields doesn't break consumers)

### 4.3 Payload Compatibility

```spec
// Producer version 1
event order_placed {
  payload {
    orderId    string
    customerId string
    totalAmount number
  }
  consumers [charge_payment]
}

// Consumer expects subset (COMPATIBLE)
behavior charge_payment {
  consumes order_placed { orderId, totalAmount }
}

// Producer version 2 (backward compatible evolution)
event order_placed {
  payload {
    orderId     string
    customerId  string
    totalAmount number
    currency    string  @optional  // New optional field
  }
  consumers [charge_payment, send_receipt]
}

// New consumer uses new field (COMPATIBLE)
behavior send_receipt {
  consumes order_placed { orderId, totalAmount, currency }
}
```

**Structural Subtyping Rules:**
- Producer MUST provide all required consumer fields
- Producer CAN provide extra fields (ignored by consumers)
- Adding `@optional` fields is backward compatible
- Removing fields is BREAKING (E041 error)

---

## 5. Contract Subtyping

### 5.1 Liskov Substitution for Behaviors

```spec
behavior base_create_user {
  contract {
    requires cmd.email != ""
    ensures result.id != ""
  }
}

behavior enhanced_create_user extends base_create_user {
  contract {
    requires cmd.email != ""         // SAME precondition (OK)
    ensures result.id != ""          // SAME postcondition
    ensures result.email == cmd.email  // STRONGER postcondition (OK)
    ensures produced(user_created)   // ADDITIONAL guarantee (OK)
  }
}

// enhanced_create_user CAN substitute base_create_user
```

**Valid Subtyping:**
- Precondition: subtype ≤ parent (accept MORE inputs)
- Postcondition: subtype ≥ parent (promise MORE guarantees)

**Invalid Subtyping:**

```spec
behavior strict_create_user extends base_create_user {
  contract {
    requires cmd.email != ""
    requires cmd.role == "admin"     // STRONGER precondition (ERROR E043!)
    ensures result.id != ""
  }
}
```

Compiler error:

```
error[E043]: behavior not substitutable (precondition strengthened)
  ┌─ behaviors/user.spec:15:5
  │
15│     requires cmd.role == "admin"
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^ this precondition is stronger than parent
  │
  = note: parent 'base_create_user' does not requires cmd.role == "admin"
  = help: relax precondition or remove 'extends' clause
```

### 5.2 Invariant Preservation

```spec
behavior base_operation {
  invariants [data_consistency, referential_integrity]

  contract {
    requires input.valid()
    ensures result.valid()
  }
}

behavior extended_operation extends base_operation {
  invariants [data_consistency, referential_integrity, additional_constraint]
  // Must preserve ALL parent invariants (checked by compiler)

  contract {
    requires input.valid()
    ensures result.valid()
    ensures result.satisfies(additional_constraint)
  }
}
```

**Type Checking Rule:**
- Subtype MUST reference all parent invariants (or prove it implies them)
- Subtype CAN add additional invariants
- Compiler emits E044 if invariant is missing

---

## 6. Port Interface Subtyping

### 6.1 Basic Port Extension

```spec
port BaseRepository {
  direction outbound

  method findById(id: string) -> Result<Entity, NotFoundError>
}

port UserRepository extends BaseRepository {
  direction outbound

  method findById(id: string) -> Result<User, NotFoundError>  // Covariant return
  method findByEmail(email: string) -> Result<User?, never>   // New method
}
```

**Subtyping Rules:**
- Subtype MUST implement all parent methods
- Subtype CAN add new methods
- Method parameters: contravariant (subtype accepts MORE general inputs)
- Method returns: covariant (subtype returns MORE specific outputs)

### 6.2 Parameter Contravariance

```spec
type Entity {
  id string
}

type User {
  id    string
  email string
}

port UserRepository {
  method create(user: User) -> Result<User, Error>
}

// VALID: Parameter more general (accepts Entity, which includes User)
port GenericRepository extends UserRepository {
  method create(entity: Entity) -> Result<User, Error>  // OK: Entity >: User
}

// INVALID: Parameter more specific
port StrictUserRepository extends UserRepository {
  method create(user: ValidatedUser) -> Result<User, Error>  // ERROR E045!
}
```

Compiler error:

```
error[E045]: port method parameter not contravariant
  ┌─ ports/user-repo.spec:12:23
  │
12│   method create(user: ValidatedUser) -> Result<User, Error>
  │                       ^^^^^^^^^^^^^^ parameter type more specific than supertype
  │
  = note: supertype method expects 'User', subtype requires 'ValidatedUser'
  = note: subtype must accept AT LEAST as general inputs as supertype
  = help: use 'User' or a supertype of 'User' for this parameter
```

### 6.3 Return Covariance

```spec
port BaseRepository {
  method findById(id: string) -> Result<Entity, NotFoundError>
}

// VALID: Return more specific (User <: Entity)
port UserRepository extends BaseRepository {
  method findById(id: string) -> Result<User, NotFoundError>  // OK: User <: Entity
}

// INVALID: Return more general
port VagueRepository extends BaseRepository {
  method findById(id: string) -> Result<object, NotFoundError>  // ERROR E046!
}
```

Compiler error:

```
error[E046]: port method return not covariant
  ┌─ ports/vague-repo.spec:8:42
  │
8 │   method findById(id: string) -> Result<object, NotFoundError>
  │                                          ^^^^^^ return type more general than supertype
  │
  = note: supertype returns 'Entity', subtype returns 'object'
  = note: subtype must return AT LEAST as specific type as supertype
  = help: use 'Entity' or a subtype of 'Entity' for return type
```

### 6.4 Error Type Covariance

```spec
type NotFoundError { ... }
type UserNotFoundError { ... }  // More specific error

port BaseRepository {
  method findById(id: string) -> Result<Entity, NotFoundError | ValidationError>
}

// VALID: Fewer error variants (more specific)
port UserRepository extends BaseRepository {
  method findById(id: string) -> Result<User, NotFoundError>  // OK: removes ValidationError
}

// INVALID: More error variants (more general)
port FailableRepository extends BaseRepository {
  method findById(id: string) -> Result<Entity, NotFoundError | ValidationError | NetworkError>
  // ERROR E046: error type not covariant (added NetworkError)
}
```

**Error Type Rules:**
- Subtype CAN return FEWER error variants (more specific)
- Subtype CANNOT return MORE error variants (more general)
- `never` is the most specific error type (no errors)

---

## 7. Set Constraints (B-Method Style)

### 7.1 Set Membership in Type Invariants

```spec
type User {
  role UserRole

  invariant "role ∈ {admin, editor, viewer}"
}

type OrderStatus {
  status string

  invariant "status ∈ {pending, confirmed, shipped, delivered, cancelled}"
}
```

**Syntax:**
```ebnf
set_membership = identifier , "∈" , "{" , literal , { "," , literal } , "}" ;
```

**Semantics:** Field value must be a member of the specified set.

### 7.2 Subset Constraints in Port Methods

```spec
port UserRepository {
  method findByRole(role: UserRole) -> Result<User[], never>
    where result ⊆ {u ∈ User | u.role == role}

  method findActive() -> Result<User[], never>
    where result ⊆ {u ∈ User | u.active == true}
    where |result| >= 0
}
```

**Syntax:**
```ebnf
where_clause = "where" , predicate ;
subset_expr  = identifier , "⊆" , set_comprehension ;
set_comprehension = "{" , identifier , "∈" , type_name , "|" , predicate , "}" ;
```

**Semantics:**
- `result ⊆ {...}` — result is a subset of the set defined by comprehension
- `{u ∈ User | u.role == role}` — set of all Users matching the predicate
- `|result|` — cardinality (size) of the result set

### 7.3 Cardinality Constraints

```spec
port UserRepository {
  method listUsers(page: integer, pageSize: integer) -> Result<User[], never>
    where |result| <= pageSize
    where |result| >= 0

  method findTopScorers(limit: integer) -> Result<User[], never>
    where |result| == min(limit, |{u ∈ User | u.score > 0}|)
}
```

**Functions:**
- `|set|` — cardinality (number of elements)
- `min(a, b)` — minimum of two values
- `max(a, b)` — maximum of two values

### 7.4 Set Operations

```spec
type Team {
  members User[]

  invariant "|members| >= 1"                    // Non-empty
  invariant "|members| <= 50"                   // Bounded
  invariant "∀ m ∈ members. m.active == true"   // All active
}

port TeamRepository {
  method findByIndustry(industry: string) -> Result<Team[], never>
    where result ⊆ {t ∈ Team | t.industry == industry}
    where ∀ t ∈ result. |t.members| >= 1       // All teams non-empty
}
```

**Quantifiers:**
- `∀` (forall) — universal quantification ("for all")
- `∃` (exists) — existential quantification ("there exists")

**Syntax:**
```ebnf
quantified_expr = quantifier , identifier , "∈" , set_expr , "." , predicate ;
quantifier = "∀" | "∃" ;
```

**Limitations (v2.6):**
- Set constraints are **validated heuristically** (pattern matching)
- Quantifiers are checked for well-formedness, not proven
- Full proof requires SMT solver (v3.0)

---

## 8. SMT Verification (v3.0, Optional)

### 8.1 Enabling SMT Verification

```bash
# Enable formal verification with Z3 solver
specforge check --verify

# Set custom timeout (default 5s per entity)
specforge check --verify --verify-timeout 10s

# Verify specific entities only
specforge check --verify --verify-only create_user,update_user
```

### 8.2 Verification Annotations

```spec
behavior transfer_funds {
  contract {
    requires from.balance >= amount.amount
    requires amount.amount > 0
    requires from.id != to.id

    ensures from.balance' == from.balance - amount.amount
    ensures to.balance' == to.balance + amount.amount
    ensures from.balance' + to.balance' == from.balance + to.balance  // Conservation
  }

  // Hint to SMT solver: assume these invariants
  verify-assume "accounts in database are consistent"
  verify-assume "no concurrent modifications"

  // Expected proof obligation
  verify-prove "balance conservation"
}
```

**Annotations:**
- `verify-assume` — assumptions for SMT solver (axioms)
- `verify-prove` — proof obligations (what to verify)

### 8.3 SMT Verification Output

```
Checking behavior 'transfer_funds' with Z3 solver...
  ✅ Precondition is satisfiable
  ✅ Postcondition implies invariants
  ✅ Balance conservation proven
  ✓ Verification successful (4.2s)

Checking behavior 'divide_amount' with Z3 solver...
  ❌ Precondition allows division by zero
  ✗ Verification failed (1.8s)

error[E049]: SMT verification failed
  ┌─ behaviors/payment.spec:42:5
  │
42│     ensures result == amount / divisor
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Z3 cannot prove divisor != 0
  │
  = note: counterexample found: amount = 100, divisor = 0
  = help: add precondition 'requires divisor != 0'
```

### 8.4 Proof Caching

```bash
# First run: full verification (slow)
specforge check --verify
# ✓ 50 entities verified (4m 32s)

# Second run: incremental (fast, only changed entities)
specforge check --verify
# ✓ 2 entities verified, 48 cached (12s)

# Force re-verification of all entities
specforge check --verify --verify-fresh
# ✓ 50 entities verified (4m 28s)
```

**Cache Strategy:**
- Proof results cached per entity in `.specforge/cache/proofs/`
- Cache key: SHA256(entity_content + dependencies)
- Invalidated when entity or dependencies change

---

## 9. Grammar Extensions Summary

### 9.1 Refinement Type Syntax (v2.2)

```ebnf
type_field        = identifier , ":" , type_expr , [ refinement ] , [ annotations ] ;
refinement        = "{" , predicate , { "," , predicate } , "}" ;
predicate         = comparison | range | set_membership | regex | derived | quantified ;
comparison        = binary_op , literal ;
binary_op         = ">" | ">=" | "<" | "<=" | "==" | "!=" ;
range             = ">=" , literal , "," , "<=" , literal ;
set_membership    = "in" , "[" , literal , { "," , literal } , "]" ;
regex             = "matches" , string_literal ;
derived           = "==" , expression ;
quantified        = quantifier , identifier , "∈" , set_expr , "." , predicate ;
quantifier        = "∀" | "∃" ;
```

### 9.2 Typed Contract Syntax (v2.2)

```ebnf
contract          = "contract" , "{" , { contract_clause } , "}" ;
contract_clause   = requires_clause | ensures_clause ;
requires_clause    = "requires" , predicate ;
ensures_clause     = "ensures" , predicate ;
predicate         = expression | built_in_predicate ;
built_in_predicate = "exists" , "(" , type_name , "," , field_match , ")"
                   | "produced" , "(" , event_id , ")"
                   | "sent" , "(" , message_id , ")" ;
```

### 9.3 Event Payload Typing Syntax (v2.3)

```ebnf
consumes_clause   = "consumes" , event_ref , [ payload_projection ] ;
payload_projection = "{" , identifier , { "," , identifier } , "}" ;
```

### 9.4 Behavior Extension Syntax (v2.4)

```ebnf
behavior_decl     = "behavior" , identifier , [ "extends" , identifier ] , [ title ] , "{" , fields , "}" ;
```

### 9.5 Port Extension Syntax (v2.5)

```ebnf
port_decl         = "port" , identifier , [ "extends" , identifier ] , [ title ] , "{" , fields , "}" ;
```

### 9.6 Set Constraint Syntax (v2.6)

```ebnf
type_invariant    = "invariant" , string_literal ;
where_clause      = "where" , predicate ;
set_comprehension = "{" , identifier , "∈" , type_name , "|" , predicate , "}" ;
subset_expr       = identifier , "⊆" , set_comprehension ;
cardinality       = "|" , identifier , "|" ;
```

---

## 10. Complete Example: E-Commerce System

See `RES-20-type-system-evolution.md`, Section 10.1 for the full 200-line example.

---

## 11. Error Message Examples

### E041: Event Payload Field Missing

```
error[E041]: event payload field missing
  ┌─ behaviors/notification.spec:8:42
  │
8 │   consumes order_placed { orderId, customerEmail }
  │                                    ^^^^^^^^^^^^^^
  │                                    field 'customerEmail' not provided by event 'order_placed'
  │
  = note: event payload includes: orderId, customerId, items, totalAmount, currency, timestamp
  = help: use 'customerId' instead, or add 'customerEmail' to event payload
```

### E042: Refinement Type Violation

```
error[E042]: refinement type violation
  ┌─ behaviors/order.spec:42:32
  │
42│     ensures order.total.amount == -100
  │                                  ^^^^ value -100 violates refinement 'amount > 0'
  │
  = note: field 'amount' in type 'Money' has refinement {v:number | v > 0}
  = help: ensure the assigned value satisfies the refinement constraint
```

### E043: Precondition Strengthened

```
error[E043]: behavior not substitutable (precondition strengthened)
  ┌─ behaviors/order.spec:18:5
  │
18│     requires cmd.items.length > 5
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  │     this precondition is stronger than parent 'place_order'
  │
  = note: parent requires 'cmd.items.length > 0'
  = note: subtype must accept AT LEAST as many inputs as parent
  = help: relax precondition to '<= parent requirement', or remove 'extends' clause
```

### E045: Parameter Not Contravariant

```
error[E045]: port method parameter not contravariant
  ┌─ ports/user-repo.spec:12:23
  │
12│   method create(user: ValidatedUser) -> Result<User, Error>
  │                       ^^^^^^^^^^^^^^ parameter type more specific than supertype
  │
  = note: supertype method expects 'User', subtype requires 'ValidatedUser'
  = note: subtype must accept AT LEAST as general inputs as supertype
  = help: use 'User' or a supertype of 'User' for this parameter
```

### W058: Possible Refinement Violation

```
warning[W058]: possible refinement violation
  ┌─ behaviors/user.spec:6:28
  │
6 │     ensures result.email == "invalid-email"
  │                            ^^^^^^^^^^^^^^^^
  │                            string literal does not match field refinement pattern
  │
  = note: field 'email' requires pattern ^[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}$
  = help: ensure the value conforms to the expected format, or add runtime validation
```

---

## 12. Migration Checklist

Upgrading existing specs to use the type system:

- [ ] **Phase 1**: Add refinements to critical types (Money, IDs, counts)
- [ ] **Phase 2**: Convert contract prose to typed `requires`/`ensures` clauses
- [ ] **Phase 3**: Add payload projections to event consumers
- [ ] **Phase 4**: Mark explicit behavior/port extension relationships
- [ ] **Phase 5**: Add set constraints to query methods
- [ ] **Phase 6** (optional): Enable `--verify` for critical behaviors

**No breaking changes** — all phases are additive. Existing specs continue to work.

---

**See also:**
- `RES-20-type-system-evolution.md` — Full research document
- `RES-20-type-system-architecture.md` — Architecture diagrams
