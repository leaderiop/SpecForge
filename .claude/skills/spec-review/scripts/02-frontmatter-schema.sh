#!/usr/bin/env bash
# 02-frontmatter-schema.sh — VAL-010 through VAL-017
# Checks: frontmatter presence, required fields, kind/status validity, recommended fields

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

SPEC_DIR=$(resolve_spec_dir "${1:-}")
INFIX=$(detect_infix "$SPEC_DIR")
IS_LEGACY=$(detect_legacy_format "$SPEC_DIR")

# ─── Directories to skip entirely ───────────────────────────────────────────

SKIP_DIRS="visual references scripts research product"

should_skip_dir() {
  local rel_dir="$1"
  local dir_name
  dir_name=$(echo "$rel_dir" | cut -d'/' -f1)
  for skip in $SKIP_DIRS; do
    [[ "$dir_name" == "$skip" ]] && return 0
  done
  return 1
}

# ─── Singleton files that may omit 'id' ─────────────────────────────────────

is_singleton() {
  local basename="$1"
  case "$basename" in
    overview.md|glossary.md|invariants.md|risk-assessment.md|traceability.md|tasks.md|urs.md|README.md)
      return 0 ;;
    *)
      return 1 ;;
  esac
}

# ─── Kind-to-directory mapping ───────────────────────────────────────────────

expected_kind_for_dir() {
  local dir_name="$1"
  case "$dir_name" in
    behaviors)    echo "behavior" ;;
    decisions)    echo "decision" ;;
    features)     echo "feature" ;;
    capabilities) echo "capability" ;;
    types)        echo "types" ;;  # some use "type", some "types"
    invariants)   echo "invariant" ;;
    process)      echo "process" ;;
    plugins)      echo "plugin" ;;
    traceability) echo "traceability" ;;
    risk-assessment) echo "risk-assessment" ;;
    roadmap)      echo "roadmap" ;;
    architecture) echo "architecture" ;;
    type-system)  echo "type-system" ;;
    compliance)   echo "compliance" ;;
    deliverables) echo "deliverable" ;;
    libraries)    echo "library" ;;
    *)            echo "" ;;
  esac
}

# ─── Valid status values per kind ────────────────────────────────────────────

valid_status_for_kind() {
  local kind="$1"
  case "$kind" in
    behavior)     echo "active deprecated draft" ;;
    decision)     echo "Accepted Superseded Draft Proposed Rejected Deprecated" ;;
    feature)      echo "active planned deprecated draft" ;;
    capability)   echo "active planned deprecated draft" ;;
    invariant)    echo "active deprecated draft" ;;
    plugin)       echo "active planned deprecated draft" ;;
    process)      echo "active deprecated draft" ;;
    deliverable)  echo "active planned deprecated draft" ;;
    library)      echo "active planned deprecated draft" ;;
    *)            echo "" ;;  # no validation for unknown kinds
  esac
}

# ─── Collect all .md files to check ─────────────────────────────────────────

find_spec_md_files() {
  local spec_dir="$1"
  # Find .md files, excluding skip dirs and index.md/index.yaml
  for f in "$spec_dir"/*.md "$spec_dir"/*/*.md "$spec_dir"/*/*/*.md; do
    [[ -f "$f" ]] || continue
    local basename_f
    basename_f=$(basename "$f")
    [[ "$basename_f" == "index.md" ]] && continue
    [[ "$basename_f" == "README.md" ]] && continue

    local rel
    rel="${f#"$spec_dir"/}"

    # Skip legacy numbered chapter files at the spec root (e.g., 01-overview.md)
    if ! echo "$rel" | grep -q '/'; then
      if echo "$basename_f" | grep -qE '^[0-9]{2}-'; then
        continue
      fi
    fi

    # Check if in a skipped directory
    if echo "$rel" | grep -q '/'; then
      local dir_part
      dir_part=$(echo "$rel" | cut -d'/' -f1)
      should_skip_dir "$dir_part" && continue
    fi

    echo "$f"
  done
}

# ─── VAL-010 through VAL-017 ────────────────────────────────────────────────

while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  rel=$(rel_path "$file" "$SPEC_DIR")
  basename_f=$(basename "$file")

  # Determine parent directory name
  dir_name=""
  if echo "$rel" | grep -q '/'; then
    dir_name=$(echo "$rel" | cut -d'/' -f1)
  fi

  # ── VAL-010: Frontmatter presence ──
  if ! has_frontmatter "$file"; then
    emit "VAL-010" "error" "$rel" "Missing YAML frontmatter (no --- fences)"
    continue
  fi

  # Extract frontmatter fields
  fm_id=$(get_fm_field "$file" "id")
  fm_kind=$(get_fm_field "$file" "kind")
  fm_title=$(get_fm_field "$file" "title")
  fm_status=$(get_fm_field "$file" "status")

  # ── VAL-011: Required fields ──
  missing=""
  if [[ -z "$fm_kind" ]]; then
    missing="${missing}kind, "
  fi
  if [[ -z "$fm_title" ]]; then
    missing="${missing}title, "
  fi
  if [[ -z "$fm_status" ]]; then
    missing="${missing}status, "
  fi
  if [[ -z "$fm_id" ]] && ! is_singleton "$basename_f"; then
    missing="${missing}id, "
  fi
  if [[ -n "$missing" ]]; then
    missing=$(echo "$missing" | sed 's/, $//')
    emit "VAL-011" "error" "$rel" "Missing required frontmatter fields: $missing"
  fi

  # ── VAL-012: kind matches parent directory ──
  if [[ -n "$fm_kind" && -n "$dir_name" ]]; then
    expected=$(expected_kind_for_dir "$dir_name")
    if [[ -n "$expected" ]]; then
      # Allow both "type" and "types" for types/ directory
      if [[ "$dir_name" == "types" ]]; then
        if [[ "$fm_kind" != "type" && "$fm_kind" != "types" ]]; then
          emit "VAL-012" "error" "$rel" "kind '$fm_kind' does not match directory '$dir_name' (expected: type or types)"
        fi
      elif [[ "$fm_kind" != "$expected" ]]; then
        emit "VAL-012" "error" "$rel" "kind '$fm_kind' does not match directory '$dir_name' (expected: $expected)"
      fi
    fi
  fi

  # ── VAL-013: status valid for kind ──
  if [[ -n "$fm_kind" && -n "$fm_status" ]]; then
    valid=$(valid_status_for_kind "$fm_kind")
    if [[ -n "$valid" ]]; then
      found=false
      for v in $valid; do
        # Case-insensitive comparison
        if [[ "${fm_status,,}" == "${v,,}" ]]; then
          found=true
          break
        fi
      done
      if [[ "$found" == "false" ]]; then
        emit "VAL-013" "warning" "$rel" "status '$fm_status' not valid for kind '$fm_kind' (valid: $valid)"
      fi
    fi
  fi

  # ── VAL-014: Behavior recommended fields ──
  if [[ "$fm_kind" == "behavior" ]]; then
    rec_missing=""
    id_range=$(get_fm_field "$file" "id_range")
    invariants=$(get_fm_list "$file" "invariants")
    adrs=$(get_fm_list "$file" "adrs")
    types=$(get_fm_list "$file" "types")
    ports=$(get_fm_list "$file" "ports")

    [[ -z "$id_range" ]] && rec_missing="${rec_missing}id_range, "
    [[ -z "$invariants" ]] && rec_missing="${rec_missing}invariants, "
    [[ -z "$adrs" ]] && rec_missing="${rec_missing}adrs, "
    [[ -z "$types" ]] && rec_missing="${rec_missing}types, "
    [[ -z "$ports" ]] && rec_missing="${rec_missing}ports, "

    if [[ -n "$rec_missing" ]]; then
      rec_missing=$(echo "$rec_missing" | sed 's/, $//')
      emit "VAL-014" "warning" "$rel" "Missing recommended behavior fields: $rec_missing"
    fi
  fi

  # ── VAL-015: Feature recommended fields ──
  if [[ "$fm_kind" == "feature" ]]; then
    rec_missing=""
    behaviors=$(get_fm_list "$file" "behaviors")
    adrs=$(get_fm_list "$file" "adrs")
    roadmap=$(get_fm_list "$file" "roadmap_phases")

    [[ -z "$behaviors" ]] && rec_missing="${rec_missing}behaviors, "
    [[ -z "$adrs" ]] && rec_missing="${rec_missing}adrs, "
    [[ -z "$roadmap" ]] && rec_missing="${rec_missing}roadmap_phases, "

    if [[ -n "$rec_missing" ]]; then
      rec_missing=$(echo "$rec_missing" | sed 's/, $//')
      emit "VAL-015" "warning" "$rel" "Missing recommended feature fields: $rec_missing"
    fi
  fi

  # ── VAL-016: Capability recommended fields ──
  if [[ "$fm_kind" == "capability" ]]; then
    rec_missing=""
    features=$(get_fm_list "$file" "features")
    behaviors=$(get_fm_list "$file" "behaviors")
    persona=$(get_fm_list "$file" "persona")
    surface=$(get_fm_list "$file" "surface")

    [[ -z "$features" ]] && rec_missing="${rec_missing}features, "
    [[ -z "$behaviors" ]] && rec_missing="${rec_missing}behaviors, "
    [[ -z "$persona" ]] && rec_missing="${rec_missing}persona, "
    [[ -z "$surface" ]] && rec_missing="${rec_missing}surface, "

    if [[ -n "$rec_missing" ]]; then
      rec_missing=$(echo "$rec_missing" | sed 's/, $//')
      emit "VAL-016" "warning" "$rel" "Missing recommended capability fields: $rec_missing"
    fi
  fi

  # ── VAL-056: Deliverable required fields ──
  if [[ "$fm_kind" == "deliverable" ]]; then
    dlv_missing=""
    dlv_type=$(get_fm_field "$file" "deliverable_type")
    caps=$(get_fm_list "$file" "capabilities")
    deps=$(get_fm_list "$file" "depends_on")

    [[ -z "$dlv_type" ]] && dlv_missing="${dlv_missing}deliverable_type, "
    [[ -z "$caps" ]] && dlv_missing="${dlv_missing}capabilities, "
    [[ -z "$deps" ]] && dlv_missing="${dlv_missing}depends_on, "

    if [[ -n "$dlv_missing" ]]; then
      dlv_missing=$(echo "$dlv_missing" | sed 's/, $//')
      emit "VAL-056" "error" "$rel" "Missing required deliverable fields: $dlv_missing"
    fi

    # Validate deliverable_type value
    if [[ -n "$dlv_type" ]]; then
      case "$dlv_type" in
        app|service|extension|cli) ;;
        *) emit "VAL-056" "error" "$rel" "Invalid deliverable_type '$dlv_type' (valid: app, service, extension, cli)" ;;
      esac
    fi
  fi

  # ── VAL-057: Library required fields ──
  if [[ "$fm_kind" == "library" ]]; then
    lib_missing=""
    npm_name=$(get_fm_field "$file" "npm_name")
    lib_path=$(get_fm_field "$file" "path")
    lib_type=$(get_fm_field "$file" "library_type")
    family=$(get_fm_field "$file" "family")
    features=$(get_fm_list "$file" "features")

    [[ -z "$npm_name" ]] && lib_missing="${lib_missing}npm_name, "
    [[ -z "$lib_path" ]] && lib_missing="${lib_missing}path, "
    [[ -z "$lib_type" ]] && lib_missing="${lib_missing}library_type, "
    [[ -z "$family" ]] && lib_missing="${lib_missing}family, "
    [[ -z "$features" ]] && lib_missing="${lib_missing}features, "

    if [[ -n "$lib_missing" ]]; then
      lib_missing=$(echo "$lib_missing" | sed 's/, $//')
      emit "VAL-057" "error" "$rel" "Missing required library fields: $lib_missing"
    fi

    # Validate library_type value
    if [[ -n "$lib_type" ]]; then
      case "$lib_type" in
        core|feature|adapter|integration|testing|tooling) ;;
        *) emit "VAL-057" "error" "$rel" "Invalid library_type '$lib_type' (valid: core, feature, adapter, integration, testing, tooling)" ;;
      esac
    fi
  fi

  # ── VAL-017: Decision recommended fields ──
  if [[ "$fm_kind" == "decision" ]]; then
    rec_missing=""
    date=$(get_fm_field "$file" "date")
    supersedes=$(get_fm_list "$file" "supersedes")

    [[ -z "$date" ]] && rec_missing="${rec_missing}date, "
    # supersedes can be empty array which is fine; check if field exists at all
    if ! grep -q '^supersedes:' "$file" 2>/dev/null; then
      rec_missing="${rec_missing}supersedes, "
    fi

    if [[ -n "$rec_missing" ]]; then
      rec_missing=$(echo "$rec_missing" | sed 's/, $//')
      emit "VAL-017" "warning" "$rel" "Missing recommended decision fields: $rec_missing"
    fi
  fi

done < <(find_spec_md_files "$SPEC_DIR")

finalize "02-frontmatter-schema (VAL-010..017, VAL-056..057)"
