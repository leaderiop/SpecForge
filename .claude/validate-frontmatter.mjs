import fs from "fs";
import path from "path";

const specDir = "spec/specforge";
const includeDirs = ["behaviors","decisions","features","capabilities","invariants","types","process","plugins","traceability","risk-assessment","roadmap","architecture"];
const rootFiles = ["overview.md","glossary.md"];
const singletonFiles = new Set(["overview.md","glossary.md"]);

const kindMap = {
  "behaviors": ["behavior"],
  "decisions": ["decision"],
  "features": ["feature"],
  "capabilities": ["capability"],
  "invariants": ["invariant"],
  "types": ["type","types"],
  "process": ["process"],
  "plugins": ["plugin"],
  "traceability": ["traceability"],
  "risk-assessment": ["risk-assessment"],
  "roadmap": ["roadmap"],
  "architecture": ["architecture"],
};

const statusMap = {
  "behaviors": ["active","deprecated"],
  "decisions": ["Accepted","Superseded","Draft"],
  "features": ["active","planned"],
  "capabilities": ["active","planned"],
};

const recommendedFields = {
  "behaviors": { rule: "VAL-014", fields: ["id_range","invariants","adrs","types","ports"] },
  "features": { rule: "VAL-015", fields: ["behaviors","adrs","roadmap_phases"] },
  "capabilities": { rule: "VAL-016", fields: ["features","behaviors","persona","surface"] },
  "decisions": { rule: "VAL-017", fields: ["date","supersedes"] },
};

const violations = {};
function addViolation(rule, file, msg) {
  if (!violations[rule]) violations[rule] = [];
  violations[rule].push({ file, msg });
}

function getFiles(dir) {
  const fullDir = path.join(specDir, dir);
  if (!fs.existsSync(fullDir)) return [];
  const entries = fs.readdirSync(fullDir, { recursive: true });
  return entries
    .filter(e => e.endsWith(".md") && path.basename(e) !== "index.md")
    .map(e => ({ rel: path.join(dir, e), full: path.join(specDir, dir, e), dir }));
}

let allFiles = [];
for (const d of includeDirs) {
  allFiles.push(...getFiles(d));
}
for (const rf of rootFiles) {
  const full = path.join(specDir, rf);
  if (fs.existsSync(full)) {
    allFiles.push({ rel: rf, full, dir: null });
  }
}

for (const { rel, full, dir } of allFiles) {
  const content = fs.readFileSync(full, "utf8");

  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  if (!fmMatch) {
    addViolation("VAL-010", rel, "No valid YAML frontmatter found");
    continue;
  }

  const fmText = fmMatch[1];
  const fm = {};
  const lines = fmText.split("\n");
  let currentKey = null;
  for (const line of lines) {
    const kvMatch = line.match(/^([\w][\w_-]*)\s*:\s*(.*)/);
    if (kvMatch) {
      currentKey = kvMatch[1];
      const val = kvMatch[2].trim();
      if (val === "" || val === "|" || val === ">") {
        fm[currentKey] = val;
      } else if (val.startsWith("[") && val.endsWith("]")) {
        fm[currentKey] = val.slice(1,-1).split(",").map(s => s.trim().replace(/^["']/,"").replace(/["']$/,""));
      } else {
        fm[currentKey] = val.replace(/^["']/,"").replace(/["']$/,"");
      }
    } else if (line.match(/^\s+-\s+/) && currentKey) {
      if (!Array.isArray(fm[currentKey])) fm[currentKey] = (fm[currentKey] && fm[currentKey] !== "") ? [fm[currentKey]] : [];
      fm[currentKey].push(line.replace(/^\s+-\s+/, "").trim().replace(/^["']/,"").replace(/["']$/,""));
    }
  }

  // VAL-011: Required fields
  const isSingleton = singletonFiles.has(path.basename(full));
  const requiredFields = isSingleton ? ["kind","title","status"] : ["id","kind","title","status"];
  const missingRequired = requiredFields.filter(f => !fm[f] || fm[f] === "");
  if (missingRequired.length > 0) {
    addViolation("VAL-011", rel, "Missing required fields: " + missingRequired.join(", "));
  }

  // VAL-012: kind matches parent directory
  if (fm.kind && dir) {
    const validKinds = kindMap[dir];
    if (validKinds && !validKinds.includes(fm.kind)) {
      addViolation("VAL-012", rel, `kind="${fm.kind}" does not match directory "${dir}" (expected: ${validKinds.join("|")})`);
    }
  }

  // VAL-013: valid status
  if (fm.status && dir && statusMap[dir]) {
    const validStatuses = statusMap[dir];
    if (!validStatuses.includes(fm.status)) {
      addViolation("VAL-013", rel, `status="${fm.status}" not valid for ${dir} (expected: ${validStatuses.join("|")})`);
    }
  }

  // VAL-014 to VAL-017: Recommended fields
  if (dir && recommendedFields[dir]) {
    const { rule, fields } = recommendedFields[dir];
    const missingRec = fields.filter(f => !fm[f]);
    if (missingRec.length > 0) {
      addViolation(rule, rel, "Missing recommended fields: " + missingRec.join(", "));
    }
  }
}

// Output
const ruleNames = {
  "VAL-010": "No valid YAML frontmatter",
  "VAL-011": "Missing required fields (id, kind, title, status)",
  "VAL-012": "kind does not match parent directory",
  "VAL-013": "Invalid status value",
  "VAL-014": "Behavior missing recommended fields (id_range, invariants, adrs, types, ports)",
  "VAL-015": "Feature missing recommended fields (behaviors, adrs, roadmap_phases)",
  "VAL-016": "Capability missing recommended fields (features, behaviors, persona, surface)",
  "VAL-017": "Decision missing recommended fields (date, supersedes)",
};

let totalViolations = 0;
const sortedRules = Object.keys(violations).sort();
for (const rule of sortedRules) {
  const vs = violations[rule];
  totalViolations += vs.length;
  console.log("");
  console.log("=".repeat(60));
  console.log(`${rule}: ${ruleNames[rule]} (${vs.length} violations)`);
  console.log("=".repeat(60));
  for (const v of vs) {
    console.log(`  ${v.file}`);
    console.log(`    -> ${v.msg}`);
  }
}

console.log("");
console.log("=".repeat(60));
console.log(`SUMMARY: ${totalViolations} total violations across ${sortedRules.length} rules`);
console.log(`Files scanned: ${allFiles.length}`);
console.log("=".repeat(60));
