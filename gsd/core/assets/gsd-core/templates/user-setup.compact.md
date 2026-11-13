<!-- Ported from gsd-core v1.19.0 gsd-core/templates/user-setup.compact.md, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD -->
# User Setup Template

Template for the `user_setup` entities of a plan - human-required configuration that Claude cannot automate.

**Purpose:** Document setup tasks that literally require human action - account creation, dashboard configuration, secret retrieval. Claude automates everything possible; a `user_setup` entity captures only what remains.

---

## The User Setup's Typed Edits

A plan's setup step is one `user_setup` entity per external service, written with `specforge new` in the plan's file
(`specforge schema --kind user_setup`; `specforge new user_setup --help` prints the fields as flags). A block list is
JSON (`--env_vars '<json>'`, `--dashboard '<json>'`), and a list of texts takes its flag once per item
(`--local_dev "<step>"`). Fill the quoted values, leave out a flag you have no value for, and repeat the line per
service with the next id (`US2`). Never write `done`: a person confirms a step with `specforge gsd confirm-setup`,
which records who and when. A refusal names its code: read it with `specforge explain <code>`.

```bash
PLAN_ID="{the plan's id, e.g. v1.0/02/02-01}"
SETUP_ID="{the plan's next setup id, e.g. US1}"
SERVICE="{the external service, e.g. stripe}"
WHY="{why the plan needs it}"
ENV_VARS='[{"name": "{ENV_VAR_NAME}", "source": "{Service Dashboard → Path → To → Value}"}]'
DASHBOARD='[{"task": "{the configuration task}", "location": "{Service Dashboard → Path → To → Setting}", "details": "{the required value and any important detail}"}]'
LOCAL_DEV="{a local development step: one --local_dev per step}"
PLAN_FILE=$(specforge query "$PLAN_ID" --depth 0 --pick '.nodes[0].file') || exit 1
specforge new user_setup "$SETUP_ID" "$SERVICE" --in "$PLAN_FILE" --plan "$PLAN_ID" --service "$SERVICE" --why "$WHY" --env_vars "$ENV_VARS" --dashboard "$DASHBOARD" --local_dev "$LOCAL_DEV" || exit 1
```

A plan's setup steps are shown with `specforge query "$PLAN_ID" --depth 1 --kinds user_setup --format context`.

---

## The Automation-First Rule

**A `user_setup` entity contains ONLY what Claude literally cannot do.**

| Claude CAN Do (no setup step) | Claude CANNOT Do (→ a setup step) |
|-----------------------------------|--------------------------------|
| `npm install stripe` | Create Stripe account |
| Write webhook handler code | Get API keys from dashboard |
| Create `.env.local` file structure | Copy actual secret values |
| Run `stripe listen` | Authenticate Stripe CLI (browser OAuth) |
| Configure package.json | Access external service dashboards |
| Write any code | Retrieve secrets from third-party systems |

**The test:** "Does this require a human in a browser, accessing an account Claude doesn't have credentials for?"
- Yes → a `user_setup` entity
- No → Claude does it automatically

---

## Guidelines

**Never include:** Actual secret values. Steps Claude can automate (package installs, code changes).

**Status tracking:** a person confirms each step with `specforge gsd confirm-setup`, which writes its `done`.
**Searchability:** `specforge export --kinds user_setup` finds every phase's user requirements.
