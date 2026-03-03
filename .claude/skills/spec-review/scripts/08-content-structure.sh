#!/usr/bin/env bash
# 08-content-structure.sh — VAL-046 through VAL-048
# Checks: BEH sections have Contract + Verification subsections, REQUIREMENT ID matches

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")

if ! has_dir "$SPEC_DIR/behaviors"; then
  skip_rule "VAL-046" "No behaviors/ directory"
  skip_rule "VAL-047" "No behaviors/ directory"
  skip_rule "VAL-048" "No behaviors/ directory"
  finalize "08-content-structure (VAL-046..048)"
  exit 0
fi

# ─── Process each behavior file ─────────────────────────────────────────────

for f in "$SPEC_DIR/behaviors"/*.md; do
  [[ -f "$f" ]] || continue
  basename_f=$(basename "$f")
  [[ "$basename_f" == "index.md" ]] && continue
  rel="behaviors/$basename_f"

  # Use awk to split by ## BEH-XX-NNN: headers and check each section
  awk -v infix="$INFIX" -v rel="$rel" '
    BEGIN {
      section_id = ""
      section_num = ""
      has_contract = 0
      has_verification = 0
      in_contract = 0
      requirement_found = 0
      requirement_id = ""
    }

    # New BEH section header
    /^## BEH-/ {
      # Check previous section
      if (section_id != "") {
        if (!has_contract) {
          printf "VAL-046\twarning\t%s\t## %s: missing ### Contract subsection\n", rel, section_id
        }
        if (has_contract && !requirement_found) {
          printf "VAL-047\terror\t%s\t## %s: ### Contract has no REQUIREMENT statement\n", rel, section_id
        }
        if (has_contract && requirement_found && requirement_id != section_num) {
          printf "VAL-047\terror\t%s\t## %s: REQUIREMENT ID BEH-%s-%s does not match section ID %s\n", rel, section_id, infix, requirement_id, section_num
        }
        if (!has_verification) {
          printf "VAL-048\twarning\t%s\t## %s: missing ### Verification subsection\n", rel, section_id
        }
      }

      # Start new section
      section_id = $0
      sub(/^## /, "", section_id)
      sub(/:.*/, "", section_id)

      # Extract numeric part
      section_num = section_id
      sub(/.*-/, "", section_num)
      # Strip leading zeros for comparison
      gsub(/^0+/, "", section_num)

      has_contract = 0
      has_verification = 0
      in_contract = 0
      requirement_found = 0
      requirement_id = ""
      next
    }

    # Detect ### Contract subsection
    /^### Contract/ {
      if (section_id != "") {
        has_contract = 1
        in_contract = 1
      }
      next
    }

    # Detect ### Verification subsection
    /^### Verification/ {
      if (section_id != "") {
        has_verification = 1
        in_contract = 0
      }
      next
    }

    # Any other ### header ends the current subsection context
    /^### / {
      in_contract = 0
      next
    }

    # Check for REQUIREMENT statement within Contract
    in_contract && /REQUIREMENT.*\(BEH-/ {
      requirement_found = 1
      # Extract the ID number from REQUIREMENT (BEH-XX-NNN):
      line = $0
      match(line, /BEH-[A-Z]+-([0-9]+)/)
      if (RSTART > 0) {
        req_match = substr(line, RSTART, RLENGTH)
        # Extract just the number
        requirement_id = req_match
        sub(/.*-/, "", requirement_id)
        gsub(/^0+/, "", requirement_id)
      }
    }

    END {
      # Check last section
      if (section_id != "") {
        if (!has_contract) {
          printf "VAL-046\twarning\t%s\t## %s: missing ### Contract subsection\n", rel, section_id
        }
        if (has_contract && !requirement_found) {
          printf "VAL-047\terror\t%s\t## %s: ### Contract has no REQUIREMENT statement\n", rel, section_id
        }
        if (has_contract && requirement_found && requirement_id != section_num) {
          printf "VAL-047\terror\t%s\t## %s: REQUIREMENT ID BEH-%s-%s does not match section ID %s\n", rel, section_id, infix, requirement_id, section_num
        }
        if (!has_verification) {
          printf "VAL-048\twarning\t%s\t## %s: missing ### Verification subsection\n", rel, section_id
        }
      }
    }
  ' "$f" | while IFS=$'\t' read -r rule severity file message; do
    [[ -z "$rule" ]] && continue
    emit "$rule" "$severity" "$file" "$message"
  done
done

# Also check plugin files
if has_dir "$SPEC_DIR/plugins"; then
  for f in "$SPEC_DIR/plugins"/*.md; do
    [[ -f "$f" ]] || continue
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    rel="plugins/$basename_f"

    # Check if this file has BEH sections
    if ! grep -q "^## BEH-${INFIX}-" "$f" 2>/dev/null; then
      continue
    fi

    awk -v infix="$INFIX" -v rel="$rel" '
      BEGIN { section_id = ""; section_num = ""; has_contract = 0; has_verification = 0; in_contract = 0; requirement_found = 0; requirement_id = "" }
      /^## BEH-/ {
        if (section_id != "") {
          if (!has_contract) printf "VAL-046\twarning\t%s\t## %s: missing ### Contract subsection\n", rel, section_id
          if (has_contract && !requirement_found) printf "VAL-047\terror\t%s\t## %s: ### Contract has no REQUIREMENT statement\n", rel, section_id
          if (has_contract && requirement_found && requirement_id != section_num) printf "VAL-047\terror\t%s\t## %s: REQUIREMENT ID mismatch\n", rel, section_id
          if (!has_verification) printf "VAL-048\twarning\t%s\t## %s: missing ### Verification subsection\n", rel, section_id
        }
        section_id = $0; sub(/^## /, "", section_id); sub(/:.*/, "", section_id)
        section_num = section_id; sub(/.*-/, "", section_num); gsub(/^0+/, "", section_num)
        has_contract = 0; has_verification = 0; in_contract = 0; requirement_found = 0; requirement_id = ""
        next
      }
      /^### Contract/ { if (section_id != "") { has_contract = 1; in_contract = 1 }; next }
      /^### Verification/ { if (section_id != "") { has_verification = 1; in_contract = 0 }; next }
      /^### / { in_contract = 0; next }
      in_contract && /REQUIREMENT.*\(BEH-/ {
        requirement_found = 1
        line = $0; match(line, /BEH-[A-Z]+-([0-9]+)/); if (RSTART > 0) { requirement_id = substr(line, RSTART, RLENGTH); sub(/.*-/, "", requirement_id); gsub(/^0+/, "", requirement_id) }
      }
      END {
        if (section_id != "") {
          if (!has_contract) printf "VAL-046\twarning\t%s\t## %s: missing ### Contract subsection\n", rel, section_id
          if (has_contract && !requirement_found) printf "VAL-047\terror\t%s\t## %s: ### Contract has no REQUIREMENT statement\n", rel, section_id
          if (has_contract && requirement_found && requirement_id != section_num) printf "VAL-047\terror\t%s\t## %s: REQUIREMENT ID mismatch\n", rel, section_id
          if (!has_verification) printf "VAL-048\twarning\t%s\t## %s: missing ### Verification subsection\n", rel, section_id
        }
      }
    ' "$f" | while IFS=$'\t' read -r rule severity file message; do
      [[ -z "$rule" ]] && continue
      emit "$rule" "$severity" "$file" "$message"
    done
  done
fi

finalize "08-content-structure (VAL-046..048)"
