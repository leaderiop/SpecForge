# Write Your First Entity

Every entity, in any domain, has the same shape: a **keyword + ID + title + fields** block. It reads like a filled-in form. The keyword names the kind, and an extension declares which kinds exist; this example uses `@specforge/software`.

## Basic syntax

```specforge
type credentials "User credentials" {
  status draft
  verify "rejects an empty email"
}

behavior login_user "User Login" {
  status   approved
  category auth
  contract "Given valid credentials, when user submits, then session is created"
  types    [credentials]

  verify unit "validates credentials format"
  verify integration "creates a session"
}
```

### Key elements

- **Keyword** -- the entity kind (`behavior`, `feature`, `event`, etc.), declared by an extension. The core language itself has only five structural keywords: `spec`, `ref`, `use`, `define`, `defaults`
- **ID** -- a unique name (words of letters, digits and underscores, joined by single `-` or `.`, 2-60 chars)
- **Title** -- a human-readable name in quotes
- **Fields** -- key-value pairs, reference lists, and verify statements
- **`use` imports** -- pull in entities from other files: `use "types/user"`

## Snippet prefixes

Type a prefix and press **Tab** to expand a full entity template. There's one per kind the builtin extensions declare:

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
  status  accepted
  problem "Users abandon carts due to complex checkout"
}

behavior process_payment "Process Payment" {
  status   approved
  category payments
  contract "Given a valid card, when the user pays, then the order is paid"
  features [checkout]
  verify unit "rejects an expired card"
}
```

This creates a `features` edge from `process_payment` to `checkout` in the graph (`feature` comes from `@specforge/product`). If `checkout` didn't exist, the check would fail with E003, unresolved reference.
