/* SpecForge — Plugin Runtime Decision renderer */
(function () {
  const D = window.PLUGIN_DATA || { dimensions: [], personas: [], matrix: [], requirements: [] };
  const $ = (s) => document.querySelector(s);
  const $$ = (s) => [...document.querySelectorAll(s)];
  const esc = (t) => String(t ?? "")
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  const VERDICT_COLORS = {
    KEEP_WASM: "#3fb950",
    LUA: "#7f7ff7",
    PYTHON: "#f7c34f",
    TYPESCRIPT: "#4fc3f7",
    MULTI: "#f78f4f",
  };

  /* minimal markdown: headings, bold, lists, code, paragraphs */
  function md(text) {
    const esc0 = esc(text);
    const lines = esc0.split("\n");
    let html = "", inList = false;
    for (const line of lines) {
      let l = line
        .replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>")
        .replace(/`([^`]+)`/g, "<code>$1</code>");
      const h = l.match(/^(#{1,4})\s+(.*)/);
      const li = l.match(/^\s*[-*]\s+(.*)/);
      if (h) {
        if (inList) { html += "</ul>"; inList = false; }
        const lvl = Math.min(h[1].length + 1, 5);
        html += `<h${lvl}>${h[2]}</h${lvl}>`;
      } else if (li) {
        if (!inList) { html += "<ul>"; inList = true; }
        html += `<li>${li[1]}</li>`;
      } else if (l.trim() === "") {
        if (inList) { html += "</ul>"; inList = false; }
      } else {
        if (inList) { html += "</ul>"; inList = false; }
        html += `<p>${l}</p>`;
      }
    }
    if (inList) html += "</ul>";
    return html;
  }

  /* ---------- header ---------- */
  $("#meta-date").textContent = D.generated_at ? "generated " + D.generated_at : "";
  $("#meta-rev").textContent = D.git_rev || "?";

  /* ---------- tabs ---------- */
  $$("#tabs .tab").forEach((b) =>
    b.addEventListener("click", () => {
      $$("#tabs .tab").forEach((x) => x.classList.remove("active"));
      $$(".view").forEach((v) => v.classList.remove("active"));
      b.classList.add("active");
      $("#view-" + b.dataset.tab).classList.add("active");
    })
  );

  /* ---------- verdict tallies ---------- */
  const tally = {};
  (D.personas || []).forEach((p) => {
    const v = (p.verdict || "?").toUpperCase();
    tally[v] = (tally[v] || 0) + 1;
  });
  const dimTally = {};
  (D.dimensions || []).forEach((dm) => {
    const v = (dm.verdict || "?").toUpperCase();
    dimTally[v] = (dimTally[v] || 0) + 1;
  });

  const totalPersonas = (D.personas || []).length;
  const totalDims = (D.dimensions || []).length;

  $("#hero-kpis").innerHTML = [
    { k: "Personas consulted", v: totalPersonas, d: "engineer-analysts" },
    { k: "Deep dives", v: totalDims, d: "decision dimensions" },
    { k: "KEEP_WASM", v: (tally.KEEP_WASM || 0) + " / " + (dimTally.KEEP_WASM || 0), d: "personas / dimensions" },
    { k: "Scripting verdicts", v: (tally.LUA || 0) + (tally.PYTHON || 0) + (tally.TYPESCRIPT || 0) + " / " + (dimTally.LUA || 0) + (dimTally.PYTHON || 0) + (dimTally.TYPESCRIPT || 0), d: "personas / dimensions" },
  ].map((x) => `<div class="kpi"><div class="k">${esc(x.k)}</div><div class="v">${esc(x.v)}</div><div class="d">${esc(x.d)}</div></div>`).join("");

  /* donut */
  (function donut() {
    const entries = Object.entries(tally).filter(([, n]) => n > 0);
    const total = entries.reduce((s, [, n]) => s + n, 0) || 1;
    let offset = 0;
    const segs = entries.map(([v, n]) => {
      const frac = n / total;
      const seg = `<circle r="15.9" cx="21" cy="21" fill="transparent" stroke="${VERDICT_COLORS[v] || "#888"}"
        stroke-width="6" stroke-dasharray="${frac * 100} ${100 - frac * 100}" stroke-dashoffset="${-offset}"></circle>`;
      offset += frac * 100;
      return seg;
    });
    $("#verdict-donut").innerHTML = segs.join("");
    $("#verdict-legend").innerHTML = entries.map(([v, n]) =>
      `<div class="row"><span class="swatch" style="background:${VERDICT_COLORS[v] || "#888"}"></span>
       <b>${v}</b>&nbsp;${n} (${Math.round((n / total) * 100)}%)</div>`).join("");
  })();

  /* dimension verdicts list */
  $("#tab-dim-count").textContent = totalDims;
  $("#dim-verdicts").innerHTML = (D.dimensions || []).map((dm) => `
    <div class="row">
      <a href="${esc(dm.file)}" target="_blank">${esc(dm.id)} — ${esc(dm.title)}</a>
      <span class="verdict-chip" style="background:${VERDICT_COLORS[(dm.verdict || "").toUpperCase()] || "#888"}22;
        color:${VERDICT_COLORS[(dm.verdict || "").toUpperCase()] || "#888"}">${esc(dm.verdict)} · conf ${esc(dm.confidence)}</span>
    </div>`).join("");

  /* recommendation */
  const rec = D.recommendation || {};
  $("#recommendation").innerHTML = md(
    (rec.headline ? `## ${rec.headline}\n` : "") + (rec.body || "")
  );

  /* requirements */
  $("#requirements").innerHTML = (D.requirements || []).map((r) =>
    `<div class="req"><b>${esc(r.id)}</b> — ${esc(r.text)}</div>`).join("");

  /* ---------- matrix ---------- */
  function renderMatrix() {
    const m = D.matrix || {};
    const rows = m.rows || [];
    $("#matrix-table").innerHTML = `<table class="matrix-table">
      <thead><tr><th>Criterion</th>${(m.options || []).map((o) =>
        `<th>${esc(o)}</th>`).join("")}</tr></thead>
      <tbody>${rows.map((r) =>
        `<tr><td class="opt">${esc(r.criterion)}</td>${(m.options || []).map((o) =>
          `<td>${md(r[o] || "—")}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
    $("#matrix-reqs").innerHTML = `<table class="matrix-table">
      <thead><tr><th>Requirement</th>${(m.options || []).map((o) =>
        `<th>${esc(o)}</th>`).join("")}</tr></thead>
      <tbody>${(D.requirements || []).map((r) => {
        const cells = (m.req_matrix && m.req_matrix[r.id]) || {};
        return `<tr><td class="opt"><b>${esc(r.id)}</b> — ${esc(r.text)}</td>${(m.options || []).map((o) => {
          const verdict = cells[o];
          const cls = verdict === "yes" ? "keep" : verdict === "partial" ? "python" : "red";
          return `<td><span class="verdict-chip" style="background:${cls}22;color:${cls}">${esc(verdict || "?")}</span></td>`;
        }).join("")}</tr>`;
      }).join("")}</tbody></table>`;
  }
  renderMatrix();

  /* ---------- dimensions tab ---------- */
  $("#tab-dim-count").textContent = totalDims;
  $("#dimensions-list").innerHTML = (D.dimensions || []).map((dm) => `
    <div class="dim-card">
      <h3>${esc(dm.id)} — ${esc(dm.title)}
        <span class="verdict-chip" style="background:${VERDICT_COLORS[(dm.verdict || "").toUpperCase()] || "#888"}22;
          color:${VERDICT_COLORS[(dm.verdict || "").toUpperCase()] || "#888"}">${esc(dm.verdict)} · conf ${esc(dm.confidence)}</span>
      </h3>
      <div class="sum">${esc(dm.summary || "")}</div>
      <div class="toggle"><a href="${esc(dm.file)}" target="_blank">open markdown →</a></div>
      <div class="full">${md(dm.markdown || "")}</div>
    </div>`).join("");

  /* ---------- personas tab ---------- */
  const personas = D.personas || [];
  $("#tab-persona-count").textContent = personas.length;

  const verdicts = [...new Set(personas.map((p) => p.verdict))].sort();
  $("#persona-verdict-filter").innerHTML =
    `<option value="">all verdicts</option>` +
    verdicts.map((v) => `<option value="${esc(v)}">${esc(v)} (${personas.filter((p) => p.verdict === v).length})</option>`).join("");
  const clusters = [...new Set(personas.map((p) => p.cluster).filter(Boolean))].sort();
  $("#persona-cluster-filter").innerHTML =
    `<option value="">all clusters</option>` +
    clusters.map((cl) => `<option value="${esc(cl)}">${esc(cl)}</option>`).join("");

  function renderPersonas() {
    const q = ($("#persona-search").value || "").toLowerCase();
    const vf = $("#persona-verdict-filter").value;
    const cf = $("#persona-cluster-filter").value;
    const shown = personas.filter((p) => {
      if (vf && p.verdict !== vf) return false;
      if (cf && p.cluster !== cf) return false;
      if (!q) return true;
      const hay = `${p.name} ${p.role} ${p.cluster} ${p.markdown || ""}`.toLowerCase();
      return hay.includes(q);
    });
    $("#persona-shown").textContent = `${shown.length} / ${personas.length} shown`;
    $("#personas-grid").innerHTML = shown.map((p) => `
      <div class="persona-card">
        <div class="head">
          <div>
            <div class="name">${esc(p.num)} — ${esc(p.name)}</div>
            <div class="role">${esc(p.role || "")} · ${esc(p.cluster || "")}</div>
          </div>
          <div style="text-align:right">
            <span class="verdict-chip" style="background:${VERDICT_COLORS[(p.verdict || "").toUpperCase()] || "#888"}22;
              color:${VERDICT_COLORS[(p.verdict || "").toUpperCase()] || "#888"}">${esc(p.verdict)}</span>
            <div class="conf">conf ${esc(p.confidence ?? "?")}/5</div>
          </div>
        </div>
        <ul class="args">${(p.arguments || []).map((a) => `<li>${esc(a)}</li>`).join("")}</ul>
        ${p.risk ? `<div class="risk"><b>risk:</b> ${esc(p.risk)}</div>` : ""}
      </div>`).join("");
  }
  $("#persona-search").addEventListener("input", renderPersonas);
  $("#persona-verdict-filter").addEventListener("change", renderPersonas);
  $("#persona-cluster-filter").addEventListener("change", renderPersonas);
  renderPersonas();

  /* ---------- evidence ---------- */
  $(".ev-rev").textContent = D.git_rev || "?";
  $("#evidence-body").innerHTML = md(D.evidence_markdown || "");
})();
