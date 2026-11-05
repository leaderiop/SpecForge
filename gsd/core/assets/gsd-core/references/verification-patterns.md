<!-- Ported from gsd-core v1.19.0 gsd-core/references/verification-patterns.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# Verification Patterns

How to verify different types of artifacts are real implementations, not stubs or placeholders.

<core_principle>
**Existence ≠ Implementation**

A file existing does not mean the feature works. Verification must check:
1. **Exists** - File is present at expected path
2. **Substantive** - Content is real implementation, not placeholder
3. **Wired** - Connected to the rest of the system
4. **Functional** - Actually works when invoked

Levels 1-3 can be checked programmatically. Level 4 often requires human verification.
</core_principle>

<stub_detection>

## Universal Stub Patterns

These patterns indicate placeholder code regardless of file type:

**Comment-based stubs:**
```bash
# Grep patterns for stub comments
specforge files search '(TODO|FIXME|XXX|HACK|PLACEHOLDER)' "$file" --format human
specforge files search -i 'implement|add later|coming soon|will be' "$file" --format human
specforge files search '// \.\.\.|/\* \.\.\. \*/|# \.\.\.' "$file" --format human
```

**Placeholder text in output:**
```bash
# UI placeholder patterns
specforge files search -i 'placeholder|lorem ipsum|coming soon|under construction' "$file" --format human
specforge files search -i 'sample|example|test data|dummy' "$file" --format human
specforge files search '\[.*\]|<.*>|\{.*\}' "$file" --format human  # Template brackets left in
```

**Empty or trivial implementations:**
```bash
# Functions that do nothing
specforge files search 'return null|return undefined|return \{\}|return \[\]' "$file" --format human
specforge files search 'pass$|\.\.\.|\bnothing\b' "$file" --format human
specforge files search 'console\.(log|warn|error).*only' "$file" --format human  # Log-only functions
```

**Hardcoded values where dynamic expected:**
```bash
# Hardcoded IDs, counts, or content
specforge files search "id.*=.*['\"].*['\"]" "$file" --format human  # Hardcoded string IDs
specforge files search 'count.*=.*\d+|length.*=.*\d+' "$file" --format human  # Hardcoded counts
specforge files search '\$\d+\.\d{2}|\d+ items' "$file" --format human  # Hardcoded display values
```

</stub_detection>

<react_components>

## React/Next.js Components

**Existence check:**
```bash
# File exists and exports component
[ -f "$component_path" ] && specforge files search 'export (default |)function|export const.*=.*\(' "$component_path" --format human
```

**Substantive check:**
```bash
# Returns actual JSX, not placeholder
specforge files search 'return.*<' "$component_path" --and-not 'return.*null' --and-not '(?i)placeholder' --format human

# Has meaningful content (not just wrapper div)
specforge files search '<[A-Z][a-zA-Z]+|className=|onClick=|onChange=' "$component_path" --format human

# Uses props or state (not static)
specforge files search 'props\.|useState|useEffect|useContext|\{.*\}' "$component_path" --format human
```

**Stub patterns specific to React:**
```javascript
// RED FLAGS - These are stubs:
return <div>Component</div>
return <div>Placeholder</div>
return <div>{/* TODO */}</div>
return <p>Coming soon</p>
return null
return <></>

// Also stubs - empty handlers:
onClick={() => {}}
onChange={() => console.log('clicked')}
onSubmit={(e) => e.preventDefault()}  // Only prevents default, does nothing
```

**Wiring check:**
```bash
# Component imports what it needs
specforge files search '^import.*from' "$component_path" --format human

# Props are actually used (not just received)
# Look for destructuring or props.X usage
specforge files search '\{ .* \}.*props|\bprops\.[a-zA-Z]+' "$component_path" --format human

# API calls exist (for data-fetching components)
specforge files search 'fetch\(|axios\.|useSWR|useQuery|getServerSideProps|getStaticProps' "$component_path" --format human
```

**Functional verification (human required):**
- Does the component render visible content?
- Do interactive elements respond to clicks?
- Does data load and display?
- Do error states show appropriately?

</react_components>

<api_routes>

## API Routes (Next.js App Router / Express / etc.)

**Existence check:**
```bash
# Route file exists
[ -f "$route_path" ]

# Exports HTTP method handlers (Next.js App Router)
specforge files search 'export (async )?(function|const) (GET|POST|PUT|PATCH|DELETE)' "$route_path" --format human

# Or Express-style handlers
specforge files search '\.(get|post|put|patch|delete)\(' "$route_path" --format human
```

**Substantive check:**
```bash
# Has actual logic, not just return statement
specforge files read "$route_path" --pick .lines  # More than 10-15 lines suggests real implementation

# Interacts with data source
specforge files search -i 'prisma\.|db\.|mongoose\.|sql|query|find|create|update|delete' "$route_path" --format human

# Has error handling
specforge files search 'try|catch|throw|error|Error' "$route_path" --format human

# Returns meaningful response
specforge files search 'Response\.json|res\.json|res\.send|return.*\{' "$route_path" --and-not '(?i)message.*not implemented' --format human
```

**Stub patterns specific to API routes:**
```typescript
// RED FLAGS - These are stubs:
export async function POST() {
  return Response.json({ message: "Not implemented" })
}

export async function GET() {
  return Response.json([])  // Empty array with no DB query
}

export async function PUT() {
  return new Response()  // Empty response
}

// Console log only:
export async function POST(req) {
  console.log(await req.json())
  return Response.json({ ok: true })
}
```

**Wiring check:**
```bash
# Imports database/service clients
specforge files search '^import.*prisma|^import.*db|^import.*client' "$route_path" --format human

# Actually uses request body (for POST/PUT)
specforge files search 'req\.json\(\)|req\.body|request\.json\(\)' "$route_path" --format human

# Validates input (not just trusting request)
specforge files search 'schema\.parse|validate|zod|yup|joi' "$route_path" --format human
```

**Functional verification (human or automated):**
- Does GET return real data from database?
- Does POST actually create a record?
- Does error response have correct status code?
- Are auth checks actually enforced?

</api_routes>

<database_schema>

## Database Schema (Prisma / Drizzle / SQL)

**Existence check:**
```bash
# Schema file exists
[ -f "prisma/schema.prisma" ] || [ -f "drizzle/schema.ts" ] || [ -f "src/db/schema.sql" ]

# Model/table is defined
specforge files search "^model $model_name|CREATE TABLE $table_name|export const $table_name" "$schema_path" --format human
```

**Substantive check:**
```bash
# Has expected fields (not just id): a field line within 20 lines of the model
specforge files search -U "model $model_name\b(.*\n){1,20}?[ \t]+\w+[ \t]+\w+" "$schema_path" --format human

# Has relationships if expected
specforge files search '@relation|REFERENCES|FOREIGN KEY' "$schema_path" --format human

# Has appropriate field types (not all String), within 20 lines of the model
specforge files search -U "model $model_name\b(.*\n){0,20}?.*(Int|DateTime|Boolean|Float|Decimal|Json)" "$schema_path" --format human
```

**Stub patterns specific to schemas:**
```prisma
// RED FLAGS - These are stubs:
model User {
  id String @id
  // TODO: add fields
}

model Message {
  id        String @id
  content   String  // Only one real field
}

// Missing critical fields:
model Order {
  id     String @id
  // No: userId, items, total, status, createdAt
}
```

**Wiring check:**
```bash
# Migrations exist and are applied
specforge files list prisma/migrations --depth 1 --no-ignore --pick .count 2>/dev/null  # Should be > 0
npx prisma migrate status 2>/dev/null | grep -v "pending"

# Client is generated
[ -d "node_modules/.prisma/client" ]
```

**Functional verification:**
```bash
# Can query the table (automated)
npx prisma db execute --stdin <<< "SELECT COUNT(*) FROM $table_name"
```

</database_schema>

<hooks_utilities>

## Custom Hooks and Utilities

**Existence check:**
```bash
# File exists and exports function
[ -f "$hook_path" ] && specforge files search 'export (default )?(function|const)' "$hook_path" --format human
```

**Substantive check:**
```bash
# Hook uses React hooks (for custom hooks)
specforge files search 'useState|useEffect|useCallback|useMemo|useRef|useContext' "$hook_path" --format human

# Has meaningful return value
specforge files search 'return \{|return \[' "$hook_path" --format human

# More than trivial length
[ "$(specforge files read "$hook_path" --pick .lines)" -gt 10 ]
```

**Stub patterns specific to hooks:**
```typescript
// RED FLAGS - These are stubs:
export function useAuth() {
  return { user: null, login: () => {}, logout: () => {} }
}

export function useCart() {
  const [items, setItems] = useState([])
  return { items, addItem: () => console.log('add'), removeItem: () => {} }
}

// Hardcoded return:
export function useUser() {
  return { name: "Test User", email: "test@example.com" }
}
```

**Wiring check:**
```bash
# Hook is actually imported somewhere ($hook_file is the hook's file name: its own file is left out)
specforge files search "import.*$hook_name" src --glob '*.tsx' --glob '*.ts' --glob "!$hook_file" --format human

# Hook is actually called
specforge files search "$hook_name\(\)" src --glob '*.tsx' --glob '*.ts' --glob "!$hook_file" --format human
```

</hooks_utilities>

<environment_config>

## Environment Variables and Configuration

**Existence check:**
```bash
# .env file exists
[ -f ".env" ] || [ -f ".env.local" ]

# Required variable is defined (in the environment: dotenv/direnv/the framework has loaded it)
printenv "$VAR_NAME" >/dev/null
```

**Substantive check:**
```bash
# Variable has an actual value (not a placeholder) -- tests the shape, never prints the value;
# exit 0 = real value, exit 1 = missing or placeholder (case-insensitive)
v=$(printenv "$VAR_NAME"); case "$(printf %s "$v" | tr '[:upper:]' '[:lower:]')" in
  ""|*your-*-here*|*xxx*|*placeholder*|*todo*) exit 1;;
esac

# Value looks valid for type:
# - URLs should start with http
# - Keys should be long enough
# - Booleans should be true/false
```

When the variable is not present in the agent's own environment (a framework that loads
`.env.local` itself at runtime does not export it to the shell that runs these checks),
ask the user to confirm it is set rather than reading `.env` directly. Variable NAMES can
still be checked against `.env.example`, which the secret-read guard exempts from its
protected-file patterns.

One guard-matching note worth knowing when auditing docs for `.env` mentions: the guard
treats a search PATTERN whose last path segment is a secret file name as a file operand, so
`specforge files search '\.env' file.md` is denied while `specforge files search '\.env\b' file.md` is allowed.

**Stub patterns specific to env:**
```bash
# RED FLAGS - These are stubs:
DATABASE_URL=your-database-url-here
STRIPE_SECRET_KEY=sk_test_xxx
API_KEY=placeholder
NEXT_PUBLIC_API_URL=http://localhost:3000  # Still pointing to localhost in prod
```

**Wiring check:**
```bash
# Variable is actually used in code
specforge files search "process\.env\.$VAR_NAME|env\.$VAR_NAME" src --glob '*.ts' --glob '*.tsx' --format human

# Variable is in validation schema (if using zod/etc for env)
specforge files search "$VAR_NAME" src/env.ts src/env.mjs --format human
```

</environment_config>

<wiring_verification>

## Wiring Verification Patterns

Wiring verification checks that components actually communicate. This is where most stubs hide.

### Pattern: Component → API

**Check:** Does the component actually call the API?

```bash
# Find the fetch/axios call
specforge files search "fetch\(['\"].*$api_path|axios\.(get|post).*$api_path" "$component_path" --format human

# Verify it's not commented out
specforge files search 'fetch\(|axios\.' "$component_path" --and-not '^.*//.*fetch' --format human

# Check the response is used
specforge files search 'await.*fetch|\.then\(|setData|setState' "$component_path" --format human
```

**Red flags:**
```typescript
// Fetch exists but response ignored:
fetch('/api/messages')  // No await, no .then, no assignment

// Fetch in comment:
// fetch('/api/messages').then(r => r.json()).then(setMessages)

// Fetch to wrong endpoint:
fetch('/api/message')  // Typo - should be /api/messages
```

### Pattern: API → Database

**Check:** Does the API route actually query the database?

```bash
# Find the database call
specforge files search "prisma\.$model|db\.query|Model\.find" "$route_path" --format human

# Verify it's awaited
specforge files search 'await.*prisma|await.*db\.' "$route_path" --format human

# Check result is returned
specforge files search 'return.*json.*data|res\.json.*result' "$route_path" --format human
```

**Red flags:**
```typescript
// Query exists but result not returned:
await prisma.message.findMany()
return Response.json({ ok: true })  // Returns static, not query result

// Query not awaited:
const messages = prisma.message.findMany()  // Missing await
return Response.json(messages)  // Returns Promise, not data
```

### Pattern: Form → Handler

**Check:** Does the form submission actually do something?

```bash
# Find onSubmit handler
specforge files search 'onSubmit=\{|handleSubmit' "$component_path" --format human

# Check handler has content: a call within 10 lines of the handler
specforge files search -U 'onSubmit.*=(.*\n){0,10}?.*(fetch|axios|mutate|dispatch)' "$component_path" --format human

# Verify not just preventDefault: a handler whose body is only `e.preventDefault()` is a stub
specforge files search -A 5 'onSubmit' "$component_path" --format human
```

**Red flags:**
```typescript
// Handler only prevents default:
onSubmit={(e) => e.preventDefault()}

// Handler only logs:
const handleSubmit = (data) => {
  console.log(data)
}

// Handler is empty:
onSubmit={() => {}}
```

### Pattern: State → Render

**Check:** Does the component render state, not hardcoded content?

```bash
# Find state usage in JSX
specforge files search '\{.*messages.*\}|\{.*data.*\}|\{.*items.*\}' "$component_path" --format human

# Check map/render of state
specforge files search '\.map\(|\.filter\(|\.reduce\(' "$component_path" --format human

# Verify dynamic content
specforge files search '\{[a-zA-Z_]+\.' "$component_path" --format human  # Variable interpolation
```

**Red flags:**
```tsx
// Hardcoded instead of state:
return <div>
  <p>Message 1</p>
  <p>Message 2</p>
</div>

// State exists but not rendered:
const [messages, setMessages] = useState([])
return <div>No messages</div>  // Always shows "no messages"

// Wrong state rendered:
const [messages, setMessages] = useState([])
return <div>{otherData.map(...)}</div>  // Uses different data
```

</wiring_verification>

<verification_checklist>

## Quick Verification Checklist

For each artifact type, run through this checklist:

### Component Checklist
- [ ] File exists at expected path
- [ ] Exports a function/const component
- [ ] Returns JSX (not null/empty)
- [ ] No placeholder text in render
- [ ] Uses props or state (not static)
- [ ] Event handlers have real implementations
- [ ] Imports resolve correctly
- [ ] Used somewhere in the app

### API Route Checklist
- [ ] File exists at expected path
- [ ] Exports HTTP method handlers
- [ ] Handlers have more than 5 lines
- [ ] Queries database or service
- [ ] Returns meaningful response (not empty/placeholder)
- [ ] Has error handling
- [ ] Validates input
- [ ] Called from frontend

### Schema Checklist
- [ ] Model/table defined
- [ ] Has all expected fields
- [ ] Fields have appropriate types
- [ ] Relationships defined if needed
- [ ] Migrations exist and applied
- [ ] Client generated

### Hook/Utility Checklist
- [ ] File exists at expected path
- [ ] Exports function
- [ ] Has meaningful implementation (not empty returns)
- [ ] Used somewhere in the app
- [ ] Return values consumed

### Wiring Checklist
- [ ] Component → API: fetch/axios call exists and uses response
- [ ] API → Database: query exists and result returned
- [ ] Form → Handler: onSubmit calls API/mutation
- [ ] State → Render: state variables appear in JSX

</verification_checklist>

<automated_verification_script>

## Automated Verification Approach

For the verification subagent, use this pattern:

```bash
# 1. Check existence
check_exists() {
  [ -f "$1" ] && echo "EXISTS: $1" || echo "MISSING: $1"
}

# 2. Check for stub patterns
check_stubs() {
  local file="$1"
  local stubs=$(specforge files search 'TODO|FIXME|placeholder|not implemented' "$file" --pick .count)
  [ "$stubs" -gt 0 ] && echo "STUB_PATTERNS: $stubs in $file"
}

# 3. Check wiring (component calls API)
check_wiring() {
  local component="$1"
  local api_path="$2"
  specforge files search -q -F "$api_path" "$component" && echo "WIRED: $component → $api_path" || echo "NOT_WIRED: $component → $api_path"
}

# 4. Check substantive (more than N lines, has expected patterns)
check_substantive() {
  local file="$1"
  local min_lines="$2"
  local pattern="$3"
  local lines; lines=$(specforge files read "$file" --pick .lines 2>/dev/null) || lines=0
  local has_pattern; has_pattern=$(specforge files search "$pattern" "$file" --pick .count 2>/dev/null) || has_pattern=0
  [ "$lines" -ge "$min_lines" ] && [ "$has_pattern" -gt 0 ] && echo "SUBSTANTIVE: $file" || echo "THIN: $file ($lines lines, $has_pattern matches)"
}
```

Run these checks against each must-have artifact. Aggregate results into VERIFICATION.md.

</automated_verification_script>

<human_verification_triggers>

## When to Require Human Verification

Some things can't be verified programmatically. Flag these for human testing:

**Always human:**
- Visual appearance (does it look right?)
- User flow completion (can you actually do the thing?)
- Real-time behavior (WebSocket, SSE)
- External service integration (Stripe, email sending)
- Error message clarity (is the message helpful?)
- Performance feel (does it feel fast?)

**Human if uncertain:**
- Complex wiring that grep can't trace
- Dynamic behavior depending on state
- Edge cases and error states
- Mobile responsiveness
- Accessibility

**Format for human verification request:**
```markdown
## Human Verification Required

### 1. Chat message sending
**Test:** Type a message and click Send
**Expected:** Message appears in list, input clears
**Check:** Does message persist after refresh?

### 2. Error handling
**Test:** Disconnect network, try to send
**Expected:** Error message appears, message not lost
**Check:** Can retry after reconnect?
```

</human_verification_triggers>

<checkpoint_automation_reference>

## Pre-Checkpoint Automation

For automation-first checkpoint patterns, server lifecycle management, CLI installation handling, and error recovery protocols, see:

**@~/.claude/gsd-core/references/checkpoints.md** → `<automation_reference>` section

Key principles:
- Claude sets up verification environment BEFORE presenting checkpoints
- Users never run CLI commands (visit URLs only)
- Server lifecycle: start before checkpoint, handle port conflicts, keep running for duration
- CLI installation: auto-install where safe, checkpoint for user choice otherwise
- Error handling: fix broken environment before checkpoint, never present checkpoint with failed setup

</checkpoint_automation_reference>
