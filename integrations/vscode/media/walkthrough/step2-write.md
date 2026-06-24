# Write Your First Entity

SpecForge uses a clean DSL with a consistent syntax: every entity is a **keyword + ID + title + fields** block.

## Basic syntax

```specforge
behavior login_user "User Login" {
  status   approved
  contract "Given valid credentials, when user submits, then session is created"
  types    [UserCredentials, Session]

  verify unit "validates credentials format"
  verify integration "creates session in database"
}
```

### Key elements

- **Keyword** -- the entity kind (`behavior`, `feature`, `event`, etc.)
- **ID** -- a unique identifier (letters, digits, underscores, 2-60 chars)
- **Title** -- a human-readable name in quotes
- **Fields** -- key-value pairs, reference lists, and verify statements
- **`use` imports** -- pull in entities from other files: `use "types/user"`

## Snippet prefixes

Type a prefix and press **Tab** to expand a full entity template:

### @specforge/software
| Prefix | Entity Kind |
|--------|------------|
| `bhv` | behavior |
| `inv` | invariant |
| `evt` | event |
| `typ` | type |
| `prt` | port |

### @specforge/product
| Prefix | Entity Kind |
|--------|------------|
| `feat` | feature |
| `jrn` | journey |
| `dlv` | deliverable |
| `mst` | milestone |
| `mod` | module |
| `trm` | term |
| `prs` | persona |
| `chn` | channel |
| `rel` | release |

### @specforge/governance
| Prefix | Entity Kind |
|--------|------------|
| `dec` | decision |
| `cst` | constraint |
| `fm` | failure_mode |

### @specforge/formal
| Prefix | Entity Kind |
|--------|------------|
| `prop` | property |
| `axm` | axiom |
| `proto` | protocol |
| `rfn` | refinement |
| `proc` | process |

### Structural
| Prefix | Block Type |
|--------|-----------|
| `spec` | spec root block |
| `use` | use import |
| `def` | define block (user-defined type) |
| `ref` | external reference |
| `vfy` | verify statement |

## Relationships via reference lists

Entities reference each other through field lists. The compiler automatically builds edges in the graph:

```specforge
feature checkout "Checkout Flow" {
  status  approved
  problem "Users abandon carts due to complex checkout"
}

behavior process_payment "Process Payment" {
  status   approved
  features [checkout]
  types    [PaymentRequest, PaymentResult]
}
```

This creates an **Implements** edge from `process_payment` to `checkout` in the entity graph.
