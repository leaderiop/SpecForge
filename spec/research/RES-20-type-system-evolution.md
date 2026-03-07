# RES-20b: Type System Evolution for SpecForge

**Research Date:** March 4, 2026
**Author:** Expert 6 (Type System Architect)
**Status:** active
**Version:** 1.0

---

## Executive Summary

This research synthesizes three formal methods traditions—**Design by Contract (DbC)**, **B-Method**, and **Communicating Sequential Processes (CSP)**—to propose a comprehensive evolution of SpecForge's type system. The goal is to make specifications more precise, enable earlier error detection, and provide stronger compile-time guarantees while maintaining the "AI agent as primary consumer" philosophy.

The proposed system adds **five layers of type-level reasoning** on top of SpecForge's existing structural types:

1. **Refinement types** on entity fields (value constraints: `amount: Money { > 0 }`)
2. **Channel typing** for event producer-consumer agreement (payload compatibility)
3. **Contract compatibility** for behavior composition (pre/postcondition subtyping)
4. **Port interface subtyping** (method signature variance rules)
5. **Phantom types** for entity ID branding (compile-time entity kind enforcement)

All five mechanisms are **opt-in, incremental, and backward compatible**. Existing specs continue to work without modification. Type refinements appear as syntactic sugar and static checks—they don't change runtime behavior or the graph structure.

---

## Table of Contents

1. [Current Baseline](#1-current-baseline)
2. [Design by Contract Integration](#2-design-by-contract-integration)
3. [B-Method Set Constraints](#3-b-method-set-constraints)
4. [CSP Channel Typing](#4-csp-channel-typing)
5. [Refinement Types on Entity Fields](#5-refinement-types-on-entity-fields)
6. [Contract Type Compatibility](#6-contract-type-compatibility)
7. [Port Interface Subtyping](#7-port-interface-subtyping)
8. [Phantom Types for Entity IDs](#8-phantom-types-for-entity-ids)
9. [Implementation Roadmap](#9-implementation-roadmap)
10. [Examples](#10-examples)
11. [Type Checker Architecture](#11-type-checker-architecture)
12. [Error Catalog](#12-error-catalog)

---

## 1. Current Baseline

### 1.1 Current Type System Features

SpecForge v1 has a **structural type system** with:

- **Primitive types**: `string`, `number`, `integer`, `boolean`, `timestamp`, `void`
- **Named types**: `type User { ... }` with struct and enum variants
- **Field annotations**: `@readonly`, `@unique`, `@optional`, `@literal`
- **Port method signatures**: `method create(cmd: CreateUserCommand) -> Result<User, Error>`
- **Event payloads**: inline struct definitions for event data
- **Entity references**: typed edges between entities (20 edge types)

### 1.2 Current Limitations

What the type system **cannot** express today:

1. **Value constraints**: "amount must be positive", "email must be valid format"
2. **Behavioral contracts**: "if precondition X holds, postcondition Y must hold"
3. **Event payload compatibility**: "does consumer expect the same fields as producer?"
4. **Port substitutability**: "can I replace PortA with PortB?"
5. **Entity ID confusion**: compiler allows passing a `BehaviorId` where a `TypeId` is expected

These gaps lead to **late error detection** (runtime, test time) instead of **early detection** (compile time, editor time).

---

## 2. Design by Contract Integration

### 2.1 DbC Core Concepts

From the research: Design by Contract uses **preconditions**, **postconditions**, and **invariants** to specify behavioral contracts. Key insights:

- **Precondition weakening**: Subtype can accept more inputs (Liskov Substitution)
- **Postcondition strengthening**: Subtype can promise more guarantees
- **Invariant preservation**: Class invariants must hold across all method calls
- **Runtime + Static Hybrid**: Eiffel uses runtime checks; modern languages (Dafny, F*) use SMT solvers

### 2.2 SpecForge's Existing Contract Model

SpecForge **already has** contract-like entities:

```spec
behavior create_user {
  invariants [unique_user_email, valid_user_role]

  contract {
    given  "valid email address not already in use"
    when   "CreateUserCommand is received"
    then   "User entity is persisted"
    ensures "user.id is generated"
    ensures "UserCreatedEvent is published"
  }
}
```

The `contract` block is **prose**. The `invariants` field references `invariant` entities, but there's no formal connection between the contract prose and the invariant semantics.

### 2.3 Proposed: Typed Contracts

**Syntax Extension:**

```spec
behavior create_user {
  invariants [unique_user_email, valid_user_role]

  contract {
    requires email.format == "valid_email"      // precondition (typed)
    requires !exists(User, email: cmd.email)    // precondition (query)

    ensures result.id != ""                     // postcondition
    ensures result.email == cmd.email           // postcondition
    ensures produced(user_created)              // side effect declaration
  }

  verify unit "creates user with generated ID"
}
```

**Semantics:**

- `requires` clauses are **preconditions** (caller's obligation)
- `ensures` clauses are **postconditions** (callee's promise)
- `produced(event_id)` declares event emission as a postcondition
- Type checker validates that:
  - All field accesses are valid (`cmd.email` exists on `CreateUserCommand`)
  - All invariant references are satisfied
  - Contract is compatible with any behaviors that call this one

**Benefits:**

1. **AI agents can reason about contracts**: LLM sees structured pre/post, not just prose
2. **Static checking**: compiler validates field references, type compatibility
3. **Test scaffold generation**: pre/post become test assertions
4. **Documentation**: contracts render to human-readable prose + formal predicates

---

## 3. B-Method Set Constraints

### 3.1 B-Method Core Concepts

From the research: The B-Method uses **set-theoretic types** with **explicit membership constraints**:

```b
MACHINE UserRegistry
VARIABLES users
INVARIANT users ⊆ USER
OPERATIONS
  add_user(u) =
    PRE u ∈ USER ∧ u ∉ users
    THEN users := users ∪ {u}
    END
END
```

Key insights:

- Types are **sets** (`USER` is the set of all valid users)
- Variables are **subsets** (`users ⊆ USER`)
- Operations have **substitution rules** (predicate transformers)
- Invariants are **membership predicates** checked after every operation

### 3.2 SpecForge Mapping: Entity Collections

SpecForge's entity model has **implicit collections**:

- `type User` defines the **set of all valid User values**
- `behavior create_user` produces members of that set
- `invariant unique_user_email` constrains the set (no two users with same email)

But there's no way to express:

- "This function returns a **subset** of users matching a predicate"
- "This invariant ensures users form a **partition** by role"
- "This collection is **bounded** (max 1000 items)"

### 3.3 Proposed: Set Constraint Annotations

**Syntax Extension:**

```spec
type User {
  id        string      @readonly
  email     string      @unique
  role      UserRole
  createdAt timestamp   @readonly

  invariant "email matches ^[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}$"
  invariant "role ∈ {admin, editor, viewer}"
}

port UserRepository {
  direction outbound

  method findByRole(role: UserRole) -> Result<User[], never>
    where result ⊆ {u ∈ User | u.role == role}
    where |result| >= 0

  method create(cmd: CreateUserCommand) -> Result<User, DuplicateEmailError>
    requires cmd.email ∉ {u.email | u ∈ existing_users}
    ensures result ∈ User
}
```

**Semantics:**

- `invariant` clause inside `type` block expresses **type-level invariant** (all instances must satisfy)
- `where result ⊆ ...` expresses **set comprehension** constraint on return value
- `|result|` is **cardinality** (collection size)
- `∈`, `∉`, `⊆` are **set operators** for membership and subset checks

**Benefits:**

1. **Stronger correctness guarantees**: compiler checks set membership constraints
2. **Query optimization hints**: `⊆` constraint helps AI agents produce efficient queries
3. **Documentation**: set constraints are precise, machine-readable specifications
4. **Test oracle generation**: set predicates become property test oracles

**Type Checker Rules:**

```rust
// Pseudo-code for set constraint validation
fn check_set_constraint(expr: &SetExpr, context: &TypeContext) -> Result<(), TypeError> {
    match expr {
        SetExpr::Membership(elem, set) => {
            let elem_type = infer_type(elem, context)?;
            let set_type = infer_type(set, context)?;
            if !elem_type.is_member_of(&set_type) {
                return Err(TypeError::MembershipViolation { elem_type, set_type });
            }
        }
        SetExpr::Subset(subset, superset) => {
            let sub_elem_type = infer_element_type(subset, context)?;
            let super_elem_type = infer_element_type(superset, context)?;
            if !sub_elem_type.is_subtype_of(&super_elem_type) {
                return Err(TypeError::SubsetViolation { subset, superset });
            }
        }
        SetExpr::Cardinality(set, bound) => {
            // Cardinality constraints are runtime checks, but we validate syntax
            validate_numeric_bound(bound, context)?;
        }
    }
    Ok(())
}
```

---

## 4. CSP Channel Typing

### 4.1 CSP Core Concepts

From the research: CSP models concurrent systems as **processes** that communicate via **channels**. Key insights:

- **Typed channels**: `c : CHAN T` — channel `c` carries values of type `T`
- **Input/output agreement**: `c!v` (send) and `c?x` (receive) must agree on type
- **Process composition**: `P || Q` requires compatible channel types at connection points
- **Refinement checking**: implementation process refines specification process if behaviors match

### 4.2 SpecForge's Event Model

SpecForge's `event` entity is **already CSP-like**:

```spec
event user_created {
  trigger   create_user         // producer process
  channel   "users.created"     // CSP channel name

  payload {                     // channel type
    userId    string
    email     string
    role      UserRole
  }

  consumers [send_notification, log_audit]  // consumer processes
}
```

But the compiler doesn't check:

- Do all consumers expect the same payload shape?
- What if a consumer expects a field that the producer doesn't send?
- What if a consumer sends to another channel—are the types compatible?

### 4.3 Proposed: Event Payload Type Checking

**Type Rule: Producer-Consumer Agreement**

```typescript
// Pseudo-type-checker logic
function checkEventPayloadCompatibility(event: Event, graph: Graph): Diagnostic[] {
    const errors: Diagnostic[] = [];
    const producerPayload = event.payload;

    for (const consumerId of event.consumers) {
        const consumer = graph.getBehavior(consumerId);
        const consumerExpectedPayload = inferConsumedEventPayload(consumer, event.id);

        if (!isStructurallyCompatible(producerPayload, consumerExpectedPayload)) {
            errors.push({
                code: "E030",
                severity: "error",
                message: `Event '${event.id}' payload mismatch: consumer '${consumerId}' expects fields ${missing_fields}`,
                span: event.span,
            });
        }
    }

    return errors;
}

function isStructurallyCompatible(producer: PayloadType, consumer: PayloadType): boolean {
    // Structural subtyping rules:
    // 1. Producer must provide all required consumer fields
    // 2. Producer can provide extra fields (ignored by consumer)
    // 3. Field types must be compatible (same primitive or same named type)
    for (const field of consumer.requiredFields()) {
        if (!producer.hasField(field.name)) {
            return false;
        }
        if (!producer.fieldType(field.name).isCompatibleWith(field.type)) {
            return false;
        }
    }
    return true;
}
```

**Syntax Extension: Consumer Payload Annotation**

```spec
behavior send_notification {
  consumes [user_created]  // implicit: expects full payload

  // OR explicit payload projection:
  consumes user_created { userId, email }  // only needs these two fields

  contract {
    requires event.userId != ""
    requires event.email.format == "valid_email"
    ensures sent(notification)
  }
}
```

**Benefits:**

1. **Early error detection**: compiler catches payload mismatches before runtime
2. **Safe refactoring**: changing event payload triggers consumer revalidation
3. **Documentation**: consumers declare which fields they care about
4. **Event versioning**: optional fields enable backward-compatible evolution

**Error Example:**

```spec
event user_created {
  payload {
    userId string
    email  string
  }
  consumers [send_welcome_email]
}

behavior send_welcome_email {
  consumes user_created { userId, email, name }  // ERROR: 'name' not in payload
}
```

Compiler error:

```
error[E030]: event payload field missing
  ┌─ behaviors/notification.spec:5:42
  │
5 │   consumes user_created { userId, email, name }
  │                                          ^^^^
  │                                          field 'name' not provided by event 'user_created'
  │
  = note: event payload only includes: userId, email
  = help: add 'name' to event payload, or remove from consumer projection
```

---

## 5. Refinement Types on Entity Fields

### 5.1 Refinement Type Theory

Refinement types extend base types with **logical predicates**:

```haskell
-- Liquid Haskell example
{-@ type Pos = {v:Int | v > 0} @-}
{-@ type NonEmptyString = {v:String | len v > 0} @-}

{-@ divide :: Int -> Pos -> Int @-}
divide :: Int -> Int -> Int
divide n d = n `div` d  -- compiler proves 'd > 0', no runtime check needed
```

Key insights:

- Refinements are **predicates on values** (not just types)
- Type checker uses **SMT solver** to prove predicate satisfaction
- Errors caught at **compile time** instead of runtime

### 5.2 Proposed: Field-Level Refinements

**Syntax:**

```spec
type Money {
  amount   number { > 0 }              // refinement: positive
  currency string { in ["USD", "EUR", "GBP"] }  // refinement: enum set
}

type User {
  id        string { len > 0 }         // non-empty
  email     string { matches "^[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}$" }
  age       integer { >= 18, <= 120 }  // range constraint
  role      UserRole
  createdAt timestamp { <= now() }     // temporal constraint
}

type OrderItem {
  quantity  integer { > 0, <= 1000 }   // bounded positive
  unitPrice number { >= 0.01 }         // minimum price
  total     number { == quantity * unitPrice }  // derived field constraint
}
```

**Refinement Expression Grammar:**

```ebnf
refinement     = "{" predicate { "," predicate } "}" ;
predicate      = comparison | range | set_membership | regex | derived ;
comparison     = binary_op , literal ;
binary_op      = ">" | ">=" | "<" | "<=" | "==" | "!=" ;
range          = ">=" , literal , "," , "<=" , literal ;
set_membership = "in" , "[" , literal , { "," , literal } , "]" ;
regex          = "matches" , string_literal ;
derived        = "==" , expression ;
expression     = field_ref | literal | binary_expr | function_call ;
```

### 5.3 Type Checking Algorithm

**Phase 1: Syntax Validation**

```rust
fn validate_refinement_syntax(refinement: &RefinementExpr, field_type: &Type) -> Result<(), TypeError> {
    match (field_type, refinement) {
        (Type::Number | Type::Integer, RefinementExpr::Comparison(op, val)) => {
            // Valid: number { > 0 }
            check_numeric_literal(val)?;
        }
        (Type::String, RefinementExpr::Regex(pattern)) => {
            // Valid: string { matches "..." }
            check_valid_regex(pattern)?;
        }
        (Type::String, RefinementExpr::Comparison(..)) => {
            // ERROR: string doesn't support > < operators
            return Err(TypeError::InvalidRefinementOperator { field_type, op });
        }
        // ... more cases
    }
    Ok(())
}
```

**Phase 2: Refinement Propagation**

```rust
fn check_refinement_satisfaction(
    usage: &FieldAccess,
    context: &TypeContext
) -> Result<(), TypeError> {
    let field_type = context.get_field_type(&usage.field_name)?;
    let refinement = context.get_field_refinement(&usage.field_name)?;

    // If the usage is in a position that expects a specific refinement,
    // check that the source refinement implies the target refinement
    if let Some(expected_refinement) = context.expected_refinement_at(usage.span) {
        if !refinement.implies(&expected_refinement) {
            return Err(TypeError::RefinementViolation {
                expected: expected_refinement,
                actual: refinement,
                span: usage.span,
            });
        }
    }

    Ok(())
}
```

**Phase 3: Contract Integration**

Refinements interact with contracts:

```spec
type PositiveMoney {
  amount   number { > 0 }
  currency string
}

behavior transfer_funds {
  contract {
    requires from.balance >= amount.amount        // compiler knows amount.amount > 0
    requires amount.amount > 0                    // REDUNDANT (refinement already ensures this)
    ensures from.balance' == from.balance - amount.amount
    ensures to.balance' == to.balance + amount.amount
  }
}
```

The compiler can **prune redundant contract clauses** if they're already enforced by refinement types.

### 5.4 Error Examples

**Error: Range Violation**

```spec
type Product {
  quantity integer { >= 0 }
}

behavior update_quantity {
  contract {
    ensures product.quantity == -5  // ERROR: refinement violated
  }
}
```

Compiler error:

```
error[E031]: refinement type violation
  ┌─ behaviors/inventory.spec:8:32
  │
8 │     ensures product.quantity == -5
  │                                ^^ value -5 violates refinement 'quantity >= 0'
  │
  = note: field 'quantity' has refinement type {v:integer | v >= 0}
  = help: ensure the assigned value satisfies the refinement constraint
```

**Error: Regex Mismatch**

```spec
type User {
  email string { matches "^[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}$" }
}

behavior create_user {
  contract {
    ensures result.email == "invalid-email"  // WARNING: likely refinement violation
  }
}
```

Compiler warning:

```
warning[W020]: possible refinement violation
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

## 6. Contract Type Compatibility

### 6.1 Liskov Substitution Principle for Behaviors

When `behavior A` references `behavior B` (via `enforced_by` or implicit composition), their contracts must be **compatible**:

- `B.precondition` ≤ `A.precondition` (B accepts more inputs)
- `B.postcondition` ≥ `A.postcondition` (B promises more)

This is **covariance/contravariance** for behavioral contracts.

### 6.2 Proposed: Contract Subtyping Rules

**Type Rule:**

```rust
fn check_behavior_substitutability(
    caller: &Behavior,
    callee: &Behavior,
    graph: &Graph,
) -> Result<(), TypeError> {
    // Rule 1: Precondition weakening (contravariance)
    // Callee can require LESS than caller expects
    for req in &callee.contract.require_clauses {
        if !caller.contract.ensure_clauses_imply(req) {
            return Err(TypeError::PreconditionStrengthened {
                caller: caller.id,
                callee: callee.id,
                violated_clause: req.clone(),
            });
        }
    }

    // Rule 2: Postcondition strengthening (covariance)
    // Callee must promise AT LEAST what caller expects
    for ensure in &caller.contract.ensure_clauses {
        if !callee.contract.ensure_clauses.contains(ensure) {
            return Err(TypeError::PostconditionWeakened {
                caller: caller.id,
                callee: callee.id,
                missing_clause: ensure.clone(),
            });
        }
    }

    // Rule 3: Invariant preservation
    // Callee must preserve all invariants that caller expects
    for inv_id in &caller.invariants {
        if !callee.invariants.contains(inv_id) && !graph.implies(callee.invariants, *inv_id) {
            return Err(TypeError::InvariantNotPreserved {
                caller: caller.id,
                callee: callee.id,
                invariant: *inv_id,
            });
        }
    }

    Ok(())
}
```

### 6.3 Example: Valid Substitution

```spec
behavior base_create_user {
  contract {
    requires cmd.email != ""
    ensures result.id != ""
  }
}

behavior enhanced_create_user {
  contract {
    requires cmd.email != ""       // SAME precondition (OK)
    ensures result.id != ""        // SAME postcondition
    ensures result.email == cmd.email  // STRONGER postcondition (OK)
    ensures produced(user_created) // ADDITIONAL guarantee (OK)
  }
}

// enhanced_create_user CAN substitute base_create_user
```

### 6.4 Example: Invalid Substitution

```spec
behavior base_create_user {
  contract {
    requires cmd.email != ""
    ensures result.id != ""
  }
}

behavior strict_create_user {
  contract {
    requires cmd.email != ""
    requires cmd.role == "admin"   // STRONGER precondition (ERROR!)
    ensures result.id != ""
  }
}

// strict_create_user CANNOT substitute base_create_user
// (rejects inputs that base_create_user would accept)
```

Compiler error:

```
error[E032]: behavior not substitutable (precondition strengthened)
  ┌─ behaviors/user.spec:15:5
  │
15│     requires cmd.role == "admin"
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^ this precondition is stronger than parent
  │
  = note: behavior 'strict_create_user' referenced by 'user_workflow'
  = note: parent 'base_create_user' does not require cmd.role == "admin"
  = help: relax precondition or mark behavior as non-substitutable
```

---

## 7. Port Interface Subtyping

### 7.1 Method Signature Variance

Port interfaces follow standard **subtyping rules**:

- **Parameter types**: contravariant (accept more general types)
- **Return types**: covariant (return more specific types)
- **Error types**: covariant (can return fewer error variants)

### 7.2 Proposed: Port Subtyping Checker

**Syntax: Port Extension**

```spec
port BaseRepository {
  direction outbound

  method findById(id: string) -> Result<Entity, NotFoundError>
}

port UserRepository extends BaseRepository {
  direction outbound

  method findById(id: string) -> Result<User, NotFoundError>  // Covariant return (User <: Entity)
  method findByEmail(email: string) -> Result<User?, never>   // Additional method
}
```

**Type Rule:**

```rust
fn check_port_subtyping(
    subtype: &Port,
    supertype: &Port,
) -> Result<(), TypeError> {
    // All methods in supertype must exist in subtype
    for super_method in &supertype.methods {
        let sub_method = subtype.get_method(&super_method.name)
            .ok_or(TypeError::MethodMissing { method: super_method.name })?;

        // Check parameter contravariance (subtype params can be MORE general)
        for (sub_param, super_param) in sub_method.params.iter().zip(&super_method.params) {
            if !is_subtype(&super_param.typ, &sub_param.typ) {
                return Err(TypeError::ParameterNotContravariant {
                    method: super_method.name,
                    param: super_param.name,
                });
            }
        }

        // Check return covariance (subtype return can be MORE specific)
        if !is_subtype(&sub_method.return_type, &super_method.return_type) {
            return Err(TypeError::ReturnNotCovariant {
                method: super_method.name,
                expected: super_method.return_type,
                actual: sub_method.return_type,
            });
        }
    }

    Ok(())
}
```

### 7.3 Example: Valid Port Subtyping

```spec
// Generic entity type
type Entity {
  id string
}

// Specific user type (subtype of Entity)
type User {
  id    string
  email string
  name  string
}

port EntityRepository {
  direction outbound
  method findById(id: string) -> Result<Entity, NotFoundError>
}

port UserRepository extends EntityRepository {
  direction outbound
  method findById(id: string) -> Result<User, NotFoundError>  // OK: User <: Entity
}

// Usage: anywhere EntityRepository is expected, UserRepository can be provided
```

### 7.4 Example: Invalid Port Subtyping

```spec
port UserRepository {
  direction outbound
  method findById(id: string) -> Result<User, NotFoundError>
}

port StrictUserRepository extends UserRepository {
  direction outbound
  // ERROR: parameter type is MORE specific (not contravariant)
  method findById(id: UserId) -> Result<User, NotFoundError>
}
```

Compiler error:

```
error[E033]: port method parameter not contravariant
  ┌─ ports/user-repo.spec:12:23
  │
12│   method findById(id: UserId) -> Result<User, NotFoundError>
  │                       ^^^^^^ parameter type more specific than supertype
  │
  = note: supertype method expects 'string', subtype requires 'UserId'
  = note: subtype must accept AT LEAST as general inputs as supertype
  = help: use 'string' or a supertype of 'string' for this parameter
```

---

## 8. ~~Phantom Types for Entity IDs~~ [Superseded: entity kinds are dynamic under zero-entity core (RES-26)]

> **Note:** This section assumes 16 static entity kinds as Rust enum variants. Under zero-entity core, entity kinds are registered dynamically from extensions via `InternedId`. Phantom type branding would need adaptation for dynamic kind registration.

### 8.1 The Entity ID Confusion Problem

Current SpecForge code has **stringly-typed entity IDs**:

```rust
// Both are just strings at compile time!
let behavior_id: EntityId = EntityId::from_str("create_user");
let type_id: EntityId = EntityId::from_str("User");

// Compiler allows this (WRONG!):
let edge = graph.add_edge(behavior_id, type_id, EdgeType::UsesType);
// Should be: graph.add_edge(behavior_id, type_id, EdgeType::UsesType)
```

The `EntityId` type carries a `kind: EntityKind` field, but it's only checked **at runtime**.

### 8.2 Proposed: Phantom Type Parameters

**Rust Implementation:**

```rust
use std::marker::PhantomData;

/// Generic entity ID with phantom type parameter for compile-time kind checking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityId<K: EntityKindMarker> {
    raw: InternedString,
    _kind: PhantomData<K>,
}

/// Marker trait for entity kinds (zero-sized types)
pub trait EntityKindMarker {
    fn kind() -> EntityKind;
}

// Zero-sized marker types for each entity kind
pub enum BehaviorMarker {}
pub enum TypeDefMarker {}
pub enum PortMarker {}
pub enum InvariantMarker {}
pub enum EventMarker {}
// ... 11 more

impl EntityKindMarker for BehaviorMarker {
    fn kind() -> EntityKind { EntityKind::Behavior }
}
impl EntityKindMarker for TypeDefMarker {
    fn kind() -> EntityKind { EntityKind::TypeDef }
}
// ... impl for other 14 kinds

/// Type aliases for ergonomics
pub type BehaviorId = EntityId<BehaviorMarker>;
pub type TypeDefId = EntityId<TypeDefMarker>;
pub type PortId = EntityId<PortMarker>;
pub type InvariantId = EntityId<InvariantMarker>;
pub type EventId = EntityId<EventMarker>;
// ... 11 more aliases

impl<K: EntityKindMarker> EntityId<K> {
    /// Create a new typed entity ID
    pub fn new(raw: impl Into<InternedString>) -> Self {
        Self {
            raw: raw.into(),
            _kind: PhantomData,
        }
    }

    /// Get the raw string ID (for serialization)
    pub fn raw(&self) -> &InternedString {
        &self.raw
    }

    /// Get the entity kind (for runtime checks)
    pub fn kind(&self) -> EntityKind {
        K::kind()
    }

    /// Erase type information (for generic storage)
    pub fn erase(self) -> ErasedEntityId {
        ErasedEntityId {
            raw: self.raw,
            kind: K::kind(),
        }
    }
}

/// Erased entity ID for storage in heterogeneous collections
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErasedEntityId {
    raw: InternedString,
    kind: EntityKind,
}

impl ErasedEntityId {
    /// Try to cast to a specific entity kind
    pub fn try_cast<K: EntityKindMarker>(&self) -> Option<EntityId<K>> {
        if self.kind == K::kind() {
            Some(EntityId {
                raw: self.raw,
                _kind: PhantomData,
            })
        } else {
            None
        }
    }

    /// Unsafe cast (for parser, when kind is validated separately)
    pub unsafe fn cast_unchecked<K: EntityKindMarker>(self) -> EntityId<K> {
        EntityId {
            raw: self.raw,
            _kind: PhantomData,
        }
    }
}
```

### 8.3 Graph API Changes

**Before (untyped):**

```rust
impl Graph {
    pub fn add_edge(&mut self, from: EntityId, to: EntityId, edge_type: EdgeType) -> EdgeIndex;
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity>;
}

// Usage (allows wrong combinations):
let behavior_id = EntityId::from_str("create_user");
let type_id = EntityId::from_str("User");
graph.add_edge(type_id, behavior_id, EdgeType::Implements);  // WRONG! (type can't implement behavior)
```

**After (typed):**

```rust
impl Graph {
    /// Add a typed edge (compile-time kind checking)
    pub fn add_edge<F, T>(
        &mut self,
        from: EntityId<F>,
        to: EntityId<T>,
        edge_type: EdgeType
    ) -> EdgeIndex
    where
        F: EntityKindMarker + ValidEdgeSource<T, EdgeType>,
        T: EntityKindMarker,
    {
        self.add_edge_erased(from.erase(), to.erase(), edge_type)
    }

    /// Get a typed entity
    pub fn get_entity<K: EntityKindMarker>(&self, id: EntityId<K>) -> Option<&Entity> {
        self.get_entity_erased(id.erase())
    }

    // Internal implementation uses erased IDs
    fn add_edge_erased(&mut self, from: ErasedEntityId, to: ErasedEntityId, edge_type: EdgeType) -> EdgeIndex;
    fn get_entity_erased(&self, id: ErasedEntityId) -> Option<&Entity>;
}

/// Trait to enforce valid edge source/target combinations at compile time
pub trait ValidEdgeSource<T: EntityKindMarker, E> {}

// Implementations for valid combinations:
impl ValidEdgeSource<BehaviorMarker, {EdgeType::Implements}> for FeatureMarker {}
impl ValidEdgeSource<InvariantMarker, {EdgeType::References}> for BehaviorMarker {}
impl ValidEdgeSource<TypeDefMarker, {EdgeType::UsesType}> for BehaviorMarker {}
impl ValidEdgeSource<EventMarker, {EdgeType::Produces}> for BehaviorMarker {}
// ... 16 more valid combinations (20 edge types total)

// Usage (compile-time checked):
let behavior_id: BehaviorId = BehaviorId::new("create_user");
let type_id: TypeDefId = TypeDefId::new("User");
graph.add_edge(behavior_id, type_id, EdgeType::UsesType);  // OK
graph.add_edge(type_id, behavior_id, EdgeType::Implements); // COMPILE ERROR!
```

**Compile Error Example:**

```rust
let type_id: TypeDefId = TypeDefId::new("User");
let behavior_id: BehaviorId = BehaviorId::new("create_user");

// This won't compile:
graph.add_edge(type_id, behavior_id, EdgeType::Implements);
//             ^^^^^^^ the trait `ValidEdgeSource<BehaviorMarker, EdgeType::Implements>`
//                     is not implemented for `TypeDefMarker`
```

### 8.4 Parser Integration

**Challenge:** Parser doesn't know entity kinds until after resolution.

**Solution:** Parser uses `ErasedEntityId`, then resolver upgrades to typed IDs:

```rust
// Parser output (erased IDs)
pub struct ParsedEntity {
    pub id: ErasedEntityId,
    pub kind: EntityKind,
    pub fields: FieldMap,
}

// Resolver casts to typed IDs after validation
impl Resolver {
    fn resolve_entity(&mut self, parsed: ParsedEntity) -> Result<TypedEntity, ResolveError> {
        let typed_id = match parsed.kind {
            EntityKind::Behavior => unsafe { parsed.id.cast_unchecked::<BehaviorMarker>() },
            EntityKind::TypeDef => unsafe { parsed.id.cast_unchecked::<TypeDefMarker>() },
            // ... 14 more cases
        };

        Ok(TypedEntity {
            id: typed_id,
            fields: parsed.fields,
        })
    }
}
```

### 8.5 Benefits

1. **Compile-time safety**: Wrong entity kind combinations rejected by Rust compiler
2. **Zero runtime cost**: Phantom types erase to nothing in compiled code
3. **Self-documenting APIs**: Function signatures show exactly what entity kinds are expected
4. **Refactoring confidence**: Changing an edge type triggers compile errors at all wrong usage sites

---

## 9. Implementation Roadmap

### Phase 1: Foundation (v2.0)

**Goal:** Add typed contracts and refinement type syntax without full checking.

- [ ] Extend grammar: `require`/`ensure` clauses in `contract` blocks
- [ ] Extend grammar: refinement expressions `{ predicate }` on type fields
- [ ] Parser recognizes new syntax, stores as opaque AST nodes
- [ ] Validator warns on unrecognized refinement operators (W021)
- [ ] Documentation: syntax guide for contracts and refinements

**Deliverable:** Specs can include typed contracts and refinements; compiler parses but doesn't validate yet.

### Phase 2: Phantom Types (v2.1)

**Goal:** Eliminate entity ID confusion at compile time.

- [ ] Replace `EntityId` with `EntityId<K: EntityKindMarker>`
- [ ] Define 16 marker types and type aliases
- [ ] Add `ValidEdgeSource<T, E>` trait with 20 implementations
- [ ] Update graph API to use typed IDs
- [ ] Update resolver to cast from erased to typed IDs
- [ ] Add `EntityId::try_cast()` for safe runtime casting
- [ ] Update tests to use typed IDs

**Deliverable:** Rust compiler rejects entity kind mismatches. All internal code uses typed IDs.

### Phase 3: Refinement Type Checker (v2.2)

**Goal:** Validate refinement constraints at compile time.

- [ ] Implement refinement expression parser (inside type checker)
- [ ] Implement `validate_refinement_syntax()` (check operators match field types)
- [ ] Implement `check_refinement_satisfaction()` (flow-sensitive checking)
- [ ] Add error codes E031 (refinement violation), W020 (possible violation)
- [ ] Integrate with contract checker (prune redundant clauses)
- [ ] Add snapshot tests for refinement errors

**Deliverable:** Compiler catches refinement violations in contracts and usage sites.

### Phase 4: Event Payload Type Checking (v2.3)

**Goal:** Validate producer-consumer payload compatibility.

- [ ] Extend grammar: consumer payload projection syntax `consumes event_id { field1, field2 }`
- [ ] Implement `checkEventPayloadCompatibility()` in validator
- [ ] Add error code E030 (payload field missing)
- [ ] Add info code I006 (consumer uses only subset of payload)
- [ ] Update event docs: payload evolution best practices
- [ ] Add snapshot tests for event payload errors

**Deliverable:** Compiler catches event payload mismatches between producers and consumers.

### Phase 5: Contract Subtyping (v2.4)

**Goal:** Validate behavior substitutability via contract checking.

- [ ] Implement `check_behavior_substitutability()` in validator
- [ ] Add error code E032 (precondition strengthened), E033 (postcondition weakened)
- [ ] Add logic for contract implication checking (SAT solver integration or heuristic)
- [ ] Integrate with `enforced_by` edge validation
- [ ] Add snapshot tests for contract compatibility errors

**Deliverable:** Compiler validates Liskov Substitution for behaviors.

### Phase 6: Port Subtyping (v2.5)

**Goal:** Validate port interface substitutability.

- [ ] Extend grammar: `port X extends Y` syntax
- [ ] Implement `check_port_subtyping()` in validator
- [ ] Add error code E034 (parameter not contravariant), E035 (return not covariant)
- [ ] Update port docs: subtyping rules and examples
- [ ] Add snapshot tests for port subtyping errors

**Deliverable:** Compiler validates port interface variance rules.

### Phase 7: Set Constraint Checker (v2.6)

**Goal:** Validate set-theoretic constraints.

- [ ] Extend grammar: `invariant` clause inside `type` blocks
- [ ] Extend grammar: `where` clause in port method signatures
- [ ] Implement `check_set_constraint()` in validator
- [ ] Add error code E036 (set membership violation), E037 (subset violation)
- [ ] Add info code I007 (set constraint as optimization hint)
- [ ] Add snapshot tests for set constraint errors

**Deliverable:** Compiler validates set membership and subset constraints.

### Phase 8: SMT Integration (v3.0)

**Goal:** Full formal verification of contracts and refinements.

- [ ] Integrate Z3 SMT solver via `z3` crate
- [ ] Implement contract-to-SMT translator
- [ ] Implement refinement-to-SMT translator
- [ ] Add `--verify` flag to enable SMT checking (opt-in)
- [ ] Add `--verify-timeout` to limit SMT solver time
- [ ] Cache SMT results per entity (incremental verification)
- [ ] Add error code E038 (SMT verification failed), W022 (SMT timeout)

**Deliverable:** Optional formal verification for contracts and refinements using SMT solver.

---

## 10. Examples

### 10.1 End-to-End Example: E-commerce Order System

```spec
// ============================================================================
// TYPES with Refinement Constraints
// ============================================================================

type Money {
  amount   number { > 0 }
  currency string { in ["USD", "EUR", "GBP"] }
}

type OrderItem {
  productId string { len > 0 }
  name      string { len > 0 }
  quantity  integer { > 0, <= 1000 }
  unitPrice Money
  total     Money { == quantity * unitPrice.amount }  // derived constraint

  invariant "total.currency == unitPrice.currency"
}

type Order {
  id         string @readonly { len > 0 }
  customerId string { len > 0 }
  items      OrderItem[] { |items| > 0, |items| <= 100 }  // non-empty, bounded
  status     OrderStatus
  total      Money
  createdAt  timestamp @readonly { <= now() }

  invariant "total.amount == sum(items.map(i => i.total.amount))"
  invariant "all items have same currency as total"
}

type OrderStatus = pending | confirmed | shipped | delivered | cancelled

// ============================================================================
// EVENTS with Payload Types
// ============================================================================

event order_placed "Order Placed" {
  trigger   place_order
  channel   "orders.placed"

  payload {
    orderId     string { len > 0 }
    customerId  string { len > 0 }
    items       OrderItem[]
    totalAmount number { > 0 }
    currency    string
    timestamp   timestamp
  }

  consumers [begin_fulfillment, charge_payment, send_order_confirm]
}

event payment_processed "Payment Processed" {
  trigger   charge_payment
  channel   "billing.payment-processed"

  payload {
    orderId       string
    paymentId     string
    amount        number { > 0 }
    currency      string
    paymentMethod string
    timestamp     timestamp
  }

  consumers [complete_fulfillment, send_payment_confirm]
}

// ============================================================================
// BEHAVIORS with Typed Contracts
// ============================================================================

behavior place_order "Place Order" {
  invariants [valid_order_items, sufficient_inventory]
  ports [order_repo, inventory_service]
  types [Order, OrderItem, Money]

  contract {
    requires cmd.items.length > 0
    requires all(cmd.items, item => item.quantity > 0)
    requires customer_exists(cmd.customerId)

    ensures result.id != ""
    ensures result.status == OrderStatus::Pending
    ensures result.total.amount == sum(cmd.items.map(i => i.total.amount))
    ensures produced(order_placed)
  }

  verify unit "creates order with pending status"
  verify unit "calculates correct total from items"
  verify integration "checks inventory availability"

  scenario "Customer places order with valid items" {
    given "Customer has account"
    given "All products are in stock"
    when  "Customer submits order with 3 items"
    then  "Order is created with status pending"
    then  "OrderPlacedEvent is published"
    then  "Inventory is reserved"
  }
}

behavior charge_payment "Charge Payment" {
  consumes order_placed { orderId, customerId, totalAmount, currency }  // payload projection
  ports [payment_gateway, order_repo]
  types [Money]

  contract {
    requires event.totalAmount > 0
    requires event.currency in ["USD", "EUR", "GBP"]
    requires customer_has_payment_method(event.customerId)

    ensures result.amount == event.totalAmount
    ensures result.currency == event.currency
    ensures order_status_updated(event.orderId, OrderStatus::Confirmed)
    ensures produced(payment_processed)
  }

  verify unit "processes payment for USD order"
  verify unit "processes payment for EUR order"
  verify integration "handles payment gateway failure"
  verify integration "retries on transient errors"
}

behavior begin_fulfillment "Begin Fulfillment" {
  consumes order_placed { orderId, items }  // only needs these fields
  ports [warehouse_service, order_repo]

  contract {
    requires event.items.length > 0
    requires all_items_in_stock(event.items)

    ensures fulfillment_task_created(event.orderId)
    ensures inventory_allocated(event.items)
  }

  verify unit "creates fulfillment task"
  verify integration "allocates inventory correctly"
}

// ============================================================================
// INVARIANTS with Set Constraints
// ============================================================================

invariant valid_order_items "Order Items Must Be Valid" {
  description "All order items must reference existing products and have positive quantities"

  enforced_by [place_order, update_order]

  rule {
    forall(order in Order,
      forall(item in order.items,
        product_exists(item.productId) &&
        item.quantity > 0 &&
        item.unitPrice.amount > 0
      )
    )
  }
}

invariant sufficient_inventory "Sufficient Inventory for Order" {
  description "Orders can only be placed if inventory is sufficient"

  enforced_by [place_order]

  rule {
    forall(order in Order,
      forall(item in order.items,
        available_inventory(item.productId) >= item.quantity
      )
    )
  }
}

// ============================================================================
// PORTS with Subtyping
// ============================================================================

port BaseRepository {
  direction outbound
  category  "persistence"

  method findById(id: string) -> Result<Entity, NotFoundError>
}

port OrderRepository extends BaseRepository {
  direction outbound
  category  "persistence/order"

  method findById(id: string) -> Result<Order, NotFoundError>  // Covariant return
  method create(order: Order) -> Result<Order, DuplicateError>
  method updateStatus(id: string, status: OrderStatus) -> Result<void, NotFoundError>
  method findByCustomer(customerId: string) -> Result<Order[], never>
    where result ⊆ {o ∈ Order | o.customerId == customerId}
}

port PaymentGateway {
  direction outbound
  category  "external/payment"

  method charge(customerId: string, amount: Money) -> Result<PaymentResult, PaymentError>
    requires amount.amount > 0
    requires amount.currency in ["USD", "EUR", "GBP"]
}
```

### 10.2 Example: Type Errors Caught at Compile Time

**Scenario 1: Refinement Violation**

```spec
behavior invalid_order {
  contract {
    ensures order.total.amount == -100  // ERROR: violates Money { > 0 }
  }
}
```

Compiler error:

```
error[E031]: refinement type violation
  ┌─ behaviors/order.spec:42:32
  │
42│     ensures order.total.amount == -100
  │                                  ^^^^ value -100 violates refinement 'amount > 0'
  │
  = note: field 'amount' in type 'Money' has refinement {v:number | v > 0}
  = help: ensure the assigned value satisfies the refinement constraint
```

**Scenario 2: Event Payload Mismatch**

```spec
behavior send_email_receipt {
  consumes order_placed { orderId, customerEmail }  // ERROR: 'customerEmail' not in payload
}
```

Compiler error:

```
error[E030]: event payload field missing
  ┌─ behaviors/notification.spec:8:42
  │
8 │   consumes order_placed { orderId, customerEmail }
  │                                    ^^^^^^^^^^^^^^
  │                                    field 'customerEmail' not provided by event 'order_placed'
  │
  = note: event payload includes: orderId, customerId, items, totalAmount, currency, timestamp
  = help: use 'customerId' instead, or add 'customerEmail' to event payload
```

**Scenario 3: Contract Incompatibility**

```spec
behavior strict_place_order extends place_order {
  contract {
    requires cmd.items.length > 5  // ERROR: precondition strengthened (parent requires > 0)
  }
}
```

Compiler error:

```
error[E032]: behavior not substitutable (precondition strengthened)
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

**Scenario 4: Port Subtyping Violation**

```spec
port RestrictedOrderRepository extends OrderRepository {
  method findById(id: OrderId) -> Result<Order, NotFoundError>  // ERROR: parameter more specific
}
```

Compiler error:

```
error[E034]: port method parameter not contravariant
  ┌─ ports/order-repo.spec:15:23
  │
15│   method findById(id: OrderId) -> Result<Order, NotFoundError>
  │                       ^^^^^^^^ parameter type more specific than supertype
  │
  = note: supertype method expects 'string', subtype requires 'OrderId'
  = note: subtype must accept AT LEAST as general inputs as supertype
  = help: use 'string' or a supertype of 'string' for this parameter
```

---

## 11. Type Checker Architecture

### 11.1 Type Checker Pipeline

```
┌─────────────┐
│   Parser    │  Produces: AST with type annotations
└──────┬──────┘
       │
       v
┌─────────────┐
│  Resolver   │  Produces: Typed graph (entities + edges)
└──────┬──────┘
       │
       v
┌─────────────┐
│ Type Checker│  NEW: Validates refinements, contracts, subtyping
└──────┬──────┘
       │
       v
┌─────────────┐
│  Validator  │  Validates: reference integrity, testability, etc.
└──────┬──────┘
       │
       v
┌─────────────┐
│  Emitter    │  Produces: Diagnostics, graph output, reports
└─────────────┘
```

### 11.2 Type Checker Components

**Module: `specforge-typechecker`**

```
crates/specforge-typechecker/
├── src/
│   ├── lib.rs                   # Public API
│   ├── context.rs               # TypeContext (type environment)
│   ├── refinement.rs            # Refinement type checking
│   ├── contract.rs              # Contract subtyping
│   ├── event.rs                 # Event payload compatibility
│   ├── port.rs                  # Port interface subtyping
│   ├── set_constraint.rs        # Set membership checking
│   ├── error.rs                 # TypeError enum
│   └── solver.rs                # Optional: SMT solver integration
└── tests/
    ├── refinement_tests.rs
    ├── contract_tests.rs
    ├── event_tests.rs
    └── port_tests.rs
```

### 11.3 TypeContext Structure

```rust
/// Type checking context: tracks type environment during validation
pub struct TypeContext {
    /// All declared types (from `type` entities)
    types: HashMap<TypeDefId, TypeInfo>,

    /// All declared ports (from `port` entities)
    ports: HashMap<PortId, PortInfo>,

    /// All behaviors (from `behavior` entities)
    behaviors: HashMap<BehaviorId, BehaviorInfo>,

    /// All events (from `event` entities)
    events: HashMap<EventId, EventInfo>,

    /// The resolved graph (for edge queries)
    graph: Arc<Graph>,

    /// String interner (for efficient string comparison)
    interner: Arc<Rodeo>,
}

pub struct TypeInfo {
    pub id: TypeDefId,
    pub fields: Vec<FieldInfo>,
    pub refinements: HashMap<String, RefinementExpr>,
    pub invariants: Vec<InvariantExpr>,
}

pub struct FieldInfo {
    pub name: String,
    pub typ: Type,
    pub annotations: FieldAnnotations,
    pub refinement: Option<RefinementExpr>,
}

pub struct PortInfo {
    pub id: PortId,
    pub direction: PortDirection,
    pub methods: Vec<MethodSignature>,
    pub extends: Option<PortId>,
}

pub struct BehaviorInfo {
    pub id: BehaviorId,
    pub contract: Option<ContractExpr>,
    pub invariants: Vec<InvariantId>,
    pub ports: Vec<PortId>,
    pub types: Vec<TypeDefId>,
}

pub struct EventInfo {
    pub id: EventId,
    pub payload: PayloadType,
    pub trigger: BehaviorId,
    pub consumers: Vec<BehaviorId>,
}
```

### 11.4 Type Checker Entry Point

```rust
impl TypeChecker {
    /// Run all type checks on the resolved graph
    pub fn check(&self, graph: &Graph) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Phase 1: Build type context from graph
        let context = TypeContext::from_graph(graph);

        // Phase 2: Check refinement types on all fields
        diagnostics.extend(self.check_refinement_types(&context));

        // Phase 3: Check event payload compatibility
        diagnostics.extend(self.check_event_payloads(&context));

        // Phase 4: Check contract subtyping for behaviors
        diagnostics.extend(self.check_contract_compatibility(&context));

        // Phase 5: Check port interface subtyping
        diagnostics.extend(self.check_port_subtyping(&context));

        // Phase 6: Check set constraints (B-Method style)
        diagnostics.extend(self.check_set_constraints(&context));

        diagnostics
    }

    fn check_refinement_types(&self, context: &TypeContext) -> Vec<Diagnostic> {
        let mut errors = Vec::new();

        for type_info in context.types.values() {
            for field in &type_info.fields {
                if let Some(refinement) = &field.refinement {
                    // Validate refinement syntax
                    if let Err(e) = validate_refinement_syntax(refinement, &field.typ) {
                        errors.push(e.to_diagnostic());
                    }
                }
            }
        }

        // Check refinement satisfaction in contracts
        for behavior in context.behaviors.values() {
            if let Some(contract) = &behavior.contract {
                for ensure_clause in &contract.ensure_clauses {
                    if let Err(e) = check_refinement_satisfaction(ensure_clause, context) {
                        errors.push(e.to_diagnostic());
                    }
                }
            }
        }

        errors
    }

    fn check_event_payloads(&self, context: &TypeContext) -> Vec<Diagnostic> {
        let mut errors = Vec::new();

        for event in context.events.values() {
            for consumer_id in &event.consumers {
                let consumer = context.behaviors.get(consumer_id).unwrap();
                let expected_payload = infer_consumed_event_payload(consumer, event.id, context);

                if !is_payload_compatible(&event.payload, &expected_payload) {
                    errors.push(Diagnostic::error(
                        "E030",
                        format!("Event '{}' payload mismatch for consumer '{}'",
                                event.id.raw(), consumer_id.raw())
                    ));
                }
            }
        }

        errors
    }

    // ... other check methods
}
```

---

## 12. Error Catalog

### 12.1 New Error Codes

| Code | Severity | Name | Description |
|------|----------|------|-------------|
| E030 | Error | Event payload field missing | Consumer expects a field not provided by producer |
| E031 | Error | Refinement type violation | Value doesn't satisfy field refinement constraint |
| E032 | Error | Precondition strengthened | Subtype behavior requires more than parent (Liskov violation) |
| E033 | Error | Postcondition weakened | Subtype behavior promises less than parent (Liskov violation) |
| E034 | Error | Parameter not contravariant | Port method parameter is more specific than parent |
| E035 | Error | Return not covariant | Port method return is more general than parent |
| E036 | Error | Set membership violation | Value not in expected set |
| E037 | Error | Subset violation | Collection not a subset of expected type |
| E038 | Error | SMT verification failed | SMT solver couldn't prove contract/refinement |

### 12.2 New Warning Codes

| Code | Severity | Name | Description |
|------|----------|------|-------------|
| W020 | Warning | Possible refinement violation | Static analysis suggests refinement might be violated |
| W021 | Warning | Unknown refinement operator | Refinement uses operator not supported for field type |
| W022 | Warning | SMT verification timeout | SMT solver timed out (inconclusive) |

### 12.3 New Info Codes

| Code | Severity | Name | Description |
|------|----------|------|-------------|
| I006 | Info | Consumer payload projection | Consumer uses only a subset of event payload (good!) |
| I007 | Info | Set constraint optimization hint | Set constraint provides query optimization opportunity |

---

## 13. Backward Compatibility

### 13.1 Zero Breaking Changes

All proposed features are **opt-in**:

- **Existing specs continue to work**: no new required syntax
- **Refinements are optional**: types without refinements are unchanged
- **Contracts can remain prose**: `contract { given/when/then }` still valid
- **Phantom types are internal**: no DSL syntax changes

### 13.2 Incremental Adoption

Projects can adopt features **gradually**:

1. **Phase 1**: Use existing types, add refinements on critical fields
2. **Phase 2**: Add typed contracts to high-risk behaviors
3. **Phase 3**: Enable event payload checking for new events
4. **Phase 4**: Add port subtyping for new ports
5. **Phase 5**: Enable full SMT verification with `--verify` flag (opt-in)

### 13.3 Migration Path

Old spec:

```spec
type Money {
  amount   number
  currency string
}

behavior transfer {
  contract {
    given "from account has sufficient balance"
    when  "transfer command is received"
    then  "funds are transferred"
  }
}
```

Upgraded spec (backward compatible):

```spec
type Money {
  amount   number { > 0 }  // NEW: refinement
  currency string { in ["USD", "EUR", "GBP"] }  // NEW: refinement
}

behavior transfer {
  contract {
    requires from.balance >= amount.amount  // NEW: typed precondition
    ensures from.balance' == from.balance - amount.amount  // NEW: typed postcondition
    ensures to.balance' == to.balance + amount.amount
  }
}
```

Both versions are valid. The compiler validates the new syntax when present, ignores it when absent.

---

## 14. Related Work

### 14.1 Similar Systems

| System | Refinements | Contracts | Channel Typing | Subtyping |
|--------|-------------|-----------|----------------|-----------|
| **Liquid Haskell** | ✅ Full SMT | ❌ No DbC | ❌ | ✅ Haskell |
| **F* / Dafny** | ✅ Full SMT | ✅ Full DbC | ❌ | ✅ Full |
| **Eiffel** | ❌ | ✅ Runtime DbC | ❌ | ✅ OO |
| **TLA+ / B-Method** | ✅ Set theory | ✅ Pre/post | ❌ | ❌ |
| **CSP / π-calculus** | ❌ | ❌ | ✅ Full | ❌ |
| **TypeScript** | ❌ | ❌ | ❌ | ✅ Structural |
| **Rust** | ❌ (traits only) | ❌ | ❌ | ✅ Nominal |
| **SpecForge (proposed)** | ✅ Lightweight | ✅ Typed DbC | ✅ Event payloads | ✅ Full |

### 14.2 SpecForge's Unique Position

SpecForge combines:

1. **Lightweight refinements** (not full dependent types)
2. **Typed contracts** (not just runtime checks)
3. **Event payload typing** (unique to event-driven specs)
4. **Phantom types** (Rust-level safety for entity IDs)
5. **AI-agent-first design** (structured contracts for LLM reasoning)

No other system targets **specification-level types for AI consumption**.

---

## 15. Conclusion

### 15.1 Summary

This research proposes **five type-level enhancements** to SpecForge:

1. **Refinement types** on entity fields for value constraints
2. **Channel typing** for event producer-consumer compatibility
3. **Contract subtyping** for behavior substitutability
4. **Port interface subtyping** for hexagonal architecture
5. **Phantom types** for compile-time entity ID safety

All features are:

- **Opt-in** (no breaking changes)
- **Incremental** (adopt feature-by-feature)
- **Backward compatible** (old specs still work)
- **Rust-native** (leverage Rust's type system for internal safety)
- **AI-friendly** (structured contracts for LLM reasoning)

### 15.2 Next Steps

1. **RES-20 Review**: Gather feedback from project stakeholders
2. **ADR Draft**: Create `ADR-TypeSystemEvolution` documenting final decisions
3. **Prototype**: Implement Phase 1 (syntax extensions) in a feature branch
4. **Validate**: Test DSL ergonomics with real-world specs
5. **Iterate**: Refine syntax based on user feedback
6. **Ship**: Release incrementally (v2.0 → v2.6 → v3.0)

### 15.3 Open Questions

1. **SMT Solver Choice**: Z3 vs. CVC5 vs. heuristic-only?
2. **Contract Syntax**: ~~`require`/`ensure` vs. `pre`/`post` vs. other keywords?~~ → **Decided: `requires`/`ensures` (with 's')**
3. **Refinement Complexity**: How much predicate logic to support?
4. **Performance**: Can type checking stay under 100ms for 10K-entity specs?
5. **Error Messages**: How to explain type errors to non-experts?

---

**End of Research Document**

*Next: Stakeholder review → ADR creation → Implementation*
