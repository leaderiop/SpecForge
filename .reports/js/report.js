/* SpecForge audit report renderer — vanilla JS, no build step */
(function () {
  "use strict";
  const D = window.AUDIT_DATA || { clusters: [], baseline: [] };
  const SEV_ORDER = ["critical", "high", "medium", "low", "info"];
  const SEV_COLOR = {
    critical: "#f85149", high: "#f0883e", medium: "#e3b341",
    low: "#58a6ff", info: "#3fb950"
  };
  const $ = (sel) => document.querySelector(sel);
  const $$ = (sel) => Array.from(document.querySelectorAll(sel));
  const esc = (s) => String(s == null ? "" : s)
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

  /* ---------- flatten ---------- */
  const clusters = Array.isArray(D.clusters) ? D.clusters : [];
  const allFindings = [];
  const allAuditors = [];
  clusters.forEach((c) => {
    (c.findings || []).forEach((f) => allFindings.push(Object.assign({ _cluster: c.cluster, _cluster_name: c.cluster_name }, f)));
    (c.auditors || []).forEach((a) => allAuditors.push(Object.assign({ _cluster: c.cluster }, a)));
  });
  const sevCount = {};
  SEV_ORDER.forEach((s) => (sevCount[s] = 0));
  allFindings.forEach((f) => {
    const s = (f.severity || "info").toLowerCase();
    sevCount[s] = (sevCount[s] || 0) + 1;
  });
  const evidenceCount = allFindings.reduce((n, f) => n + ((f.evidence || []).length || (f.evidence ? 1 : 0)), 0);

  /* ---------- header meta ---------- */
  $("#meta-date").textContent = D.generated_at ? "generated " + D.generated_at : "";
  $("#meta-rev").textContent = D.git_rev || "?";
  $("#tab-findings-count").textContent = allFindings.length;
  $("#tab-auditors-count").textContent = allAuditors.length;
  $("#foot-counts").textContent =
    allFindings.length + " findings · " + evidenceCount + " evidence items · " +
    allAuditors.length + " auditors · " + clusters.length + " clusters";

  /* ---------- tabs ---------- */
  $$("#tabs .tab").forEach((btn) => {
    btn.addEventListener("click", () => {
      $$("#tabs .tab").forEach((b) => b.classList.remove("active"));
      $$(".view").forEach((v) => v.classList.remove("active"));
      btn.classList.add("active");
      $("#view-" + btn.dataset.tab).classList.add("active");
      if (btn.dataset.tab === "diagrams") renderDiagramsOnce();
    });
  });

  /* ---------- dashboard hero KPIs ---------- */
  const kpiStatus = { good: 0, warn: 0, bad: 0 };
  clusters.forEach((c) => (c.kpis || []).forEach((k) => { kpiStatus[k.status] = (kpiStatus[k.status] || 0) + 1; }));
  const totalKpis = kpiStatus.good + kpiStatus.warn + kpiStatus.bad;
  const critHigh = sevCount.critical + sevCount.high;
  const hero = [
    { label: "Findings", value: allFindings.length, hint: "across " + clusters.length + " audit clusters", cls: "" },
    { label: "Critical + high", value: critHigh, hint: "must-fix issues", cls: critHigh ? "bad" : "good" },
    { label: "KPIs tracked", value: totalKpis, hint: kpiStatus.good + " good · " + kpiStatus.warn + " warn · " + kpiStatus.bad + " bad", cls: kpiStatus.bad ? "warn" : "good" },
    { label: "Evidence items", value: evidenceCount, hint: "file-backed citations", cls: "" },
    { label: "Auditors", value: allAuditors.length, hint: "each with a dedicated review page", cls: "" },
    { label: "Diagrams", value: clusters.reduce((n, c) => n + (c.diagrams || []).length, 0), hint: "cluster-level explanations", cls: "" }
  ];
  $("#hero-kpis").innerHTML = hero.map((k) => `
    <div class="kpi ${k.cls}">
      <div class="label">${esc(k.label)}</div>
      <div class="value">${esc(k.value)}</div>
      <div class="hint">${esc(k.hint)}</div>
    </div>`).join("");

  /* ---------- severity donut ---------- */
  (function donut() {
    const svg = $("#severity-donut");
    const total = allFindings.length || 1;
    const C = 2 * Math.PI * 15.9155;
    let offset = 0;
    let segs = "";
    SEV_ORDER.forEach((s) => {
      if (!sevCount[s]) return;
      const frac = sevCount[s] / total;
      const dash = `${(frac * C).toFixed(3)} ${(C - frac * C).toFixed(3)}`;
      segs += `<circle r="15.9155" cx="21" cy="21" fill="transparent" stroke="${SEV_COLOR[s]}"
        stroke-width="5.5" stroke-dasharray="${dash}" stroke-dashoffset="${-offset.toFixed(3)}"></circle>`;
      offset += frac * C;
    });
    svg.innerHTML = segs +
      `<text x="21" y="20" text-anchor="middle" transform="rotate(90 21 21)" fill="#e6edf3" font-size="6" font-weight="700">${total}</text>
       <text x="21" y="25.5" text-anchor="middle" transform="rotate(90 21 21)" fill="#8b98a9" font-size="2.6">findings</text>`;
    $("#severity-legend").innerHTML = SEV_ORDER.map((s) => `
      <div class="row"><span class="swatch" style="background:${SEV_COLOR[s]}"></span>
      <b>${s}</b><span class="muted">· ${sevCount[s] || 0}</span></div>`).join("");
  })();

  /* ---------- cluster bars ---------- */
  (function bars() {
    const svg = $("#cluster-bars");
    const data = clusters.map((c) => ({
      name: c.cluster,
      n: (c.findings || []).length,
      ch: (c.findings || []).filter((f) => ["critical", "high"].includes((f.severity || "").toLowerCase())).length
    }));
    const W = 620, H = 260, padL = 34, padB = 34, padT = 14;
    const max = Math.max(1, ...data.map((d) => d.n));
    const bw = (W - padL - 10) / Math.max(1, data.length);
    let out = `<line class="axis" x1="${padL}" y1="${padT}" x2="${padL}" y2="${H - padB}"></line>
               <line class="axis" x1="${padL}" y1="${H - padB}" x2="${W}" y2="${H - padB}"></line>`;
    data.forEach((d, i) => {
      const h = ((d.n - 0) / max) * (H - padB - padT - 10);
      const ch = (d.ch / max) * (H - padB - padT - 10);
      const x = padL + i * bw + bw * 0.18;
      const w = bw * 0.64;
      out += `<rect x="${x}" y="${H - padB - h}" width="${w}" height="${Math.max(h, 1)}" rx="3"
                 fill="#1f6feb" opacity="0.85"><title>${d.name}: ${d.n} findings</title></rect>`;
      out += `<rect x="${x}" y="${H - padB - ch}" width="${w}" height="${Math.max(ch, 0.5)}" rx="3"
                 fill="#f85149" opacity="0.9"><title>${d.name}: ${d.ch} critical/high</title></rect>`;
      out += `<text class="bar-value" x="${x + w / 2}" y="${H - padB - h - 5}" text-anchor="middle">${d.n}</text>`;
      out += `<text class="bar-label" x="${x + w / 2}" y="${H - padB + 14}" text-anchor="middle">${esc(d.name)}</text>`;
    });
    out += `<text class="bar-label" x="${padL}" y="${padT + 2}">count</text>`;
    svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
    svg.innerHTML = out;
  })();

  /* ---------- top risks ---------- */
  (function risks() {
    const top = allFindings
      .filter((f) => ["critical", "high"].includes((f.severity || "").toLowerCase()))
      .slice(0, 12);
    $("#top-risks").innerHTML = top.length
      ? top.map((f) => `
        <div class="risk-row">
          <span class="chip ${esc(f.severity)}">${esc(f.severity)}</span>
          <span class="t">${esc(f.title)}<span class="c"> — ${esc(f.engineer || "")} · ${esc(f._cluster || "")}</span></span>
        </div>`).join("")
      : '<span class="muted">No critical or high findings.</span>';
  })();

  /* ---------- baseline ---------- */
  (function baseline() {
    const items = Array.isArray(D.baseline) ? D.baseline : [];
    $("#baseline-grid").innerHTML = items.map((b) => `
      <div class="baseline-item">
        <div class="k">${esc(b.k)}</div>
        <div class="v">${esc(b.v)}</div>
      </div>`).join("");
  })();

  /* ---------- findings tab ---------- */
  const state = { sevs: new Set(SEV_ORDER), cluster: "", q: "" };
  const sevFilter = $("#sev-filter");
  sevFilter.innerHTML = SEV_ORDER.map((s) =>
    `<span class="sev-btn on ${s}" data-sev="${s}">${s} (${sevCount[s] || 0})</span>`).join("");
  $$("#sev-filter .sev-btn").forEach((b) =>
    b.addEventListener("click", () => {
      const s = b.dataset.sev;
      if (state.sevs.has(s)) { state.sevs.delete(s); b.classList.remove("on"); }
      else { state.sevs.add(s); b.classList.add("on"); }
      renderFindings();
    }));
  const sel = $("#cluster-filter");
  clusters.forEach((c) => {
    const o = document.createElement("option");
    o.value = c.cluster; o.textContent = `${c.cluster} — ${c.cluster_name || ""}`;
    sel.appendChild(o);
  });
  sel.addEventListener("change", () => { state.cluster = sel.value; renderFindings(); });
  $("#search").addEventListener("input", (e) => { state.q = e.target.value.toLowerCase(); renderFindings(); });

  function matches(f) {
    if (!state.sevs.has((f.severity || "info").toLowerCase())) return false;
    if (state.cluster && f._cluster !== state.cluster) return false;
    if (state.q) {
      const hay = [f.title, f.engineer, f.area, f.solution, f.eli10, f.details, f._cluster_name,
        ...(f.evidence || []).map((e) => (typeof e === "string" ? e : [e.path, e.lines, e.what].join(" ")))]
        .join(" ").toLowerCase();
      if (!hay.includes(state.q)) return false;
    }
    return true;
  }

  function renderFindings() {
    const list = allFindings.filter(matches);
    $("#findings-shown").textContent = `${list.length} / ${allFindings.length} shown`;
    const order = { critical: 0, high: 1, medium: 2, low: 3, info: 4 };
    list.sort((a, b) => (order[a.severity] ?? 9) - (order[b.severity] ?? 9));
    $("#findings-list").innerHTML = list.map((f) => {
      const ev = (f.evidence || []).map((e) => {
        if (typeof e === "string") return `<span class="ev"><code>${esc(e)}</code></span>`;
        const loc = e.path ? `<code>${esc(e.path)}${e.lines ? ":" + esc(e.lines) : ""}</code>` : "";
        return `<span class="ev">${loc}${e.what ? " — " + esc(e.what) : ""}</span>`;
      }).join("");
      const firstNum = (f.engineer || "").match(/\b(\d{3})\b/);
      const page = firstNum && D.page_of ? D.page_of[firstNum[1]] : null;
      const link = page ? `<a class="engineer" style="margin-left:auto" href="agents/${esc(page)}.html#f-${esc(f.fid || "")}">Full review →</a>` : "";
      return `
      <article class="finding sev-${esc((f.severity || "info").toLowerCase())}">
        <div class="head">
          <h3>${esc(f.title)}</h3>
          <span class="chip ${esc((f.severity || "info").toLowerCase())}">${esc(f.severity)}</span>
          <span class="area">${esc(f.area || "")} · ${esc(f._cluster || "")}</span>
        </div>
        <div class="meta-row">
          <span class="engineer">${esc(f.engineer || "")}</span>
          ${f.effort ? `<span class="chip neutral">effort ${esc(f.effort)}</span>` : ""}
          ${f.kpi_impact ? `<span class="chip neutral">moves: ${esc(f.kpi_impact)}</span>` : ""}
          ${link}
        </div>
        ${f.details ? `<p style="font-size:13.5px;margin:6px 0">${esc(f.details)}</p>` : ""}
        ${ev ? `<div class="evidence"><b>Evidence</b>${ev}</div>` : ""}
        <p class="sol"><b>Proposed solution.</b> ${esc(f.solution || "")}</p>
        <div class="eli10"><span class="tag">Explain like I'm 10</span><p>${esc(f.eli10 || "")}</p></div>
      </article>`;
    }).join("");
  }
  renderFindings();

  /* ---------- KPIs & metrics tab ---------- */
  (function kpis() {
    $("#kpis-content").innerHTML = clusters.map((c) => {
      const kpiRows = (c.kpis || []).map((k) => `
        <tr><td>${esc(k.name)}</td><td><b>${esc(k.value)}${k.unit ? " " + esc(k.unit) : ""}</b></td>
        <td><span class="chip ${esc(k.status)}">${esc(k.status)}</span></td>
        <td><code>${esc(k.evidence || "")}</code></td></tr>`).join("");
      const metRows = (c.metrics || []).map((m) => `
        <tr><td>${esc(m.name)}</td><td><b>${esc(m.value)}</b></td>
        <td>${esc(m.how_measured || "")}</td><td><code>${esc(m.evidence || "")}</code></td></tr>`).join("");
      return `
      <div class="cluster-block">
        <h2>${esc(c.cluster)} — ${esc(c.cluster_name || "")} <span class="muted" style="font-weight:400;font-size:13px">· ${esc(c.subsystem || "")}</span></h2>
        <table class="kpi-table">
          <thead><tr><th>KPI</th><th>Value</th><th>Status</th><th>Evidence</th></tr></thead>
          <tbody>${kpiRows || '<tr><td colspan="4" class="muted">none</td></tr>'}</tbody>
        </table>
        <table class="kpi-table">
          <thead><tr><th>Metric</th><th>Value</th><th>How measured</th><th>Evidence</th></tr></thead>
          <tbody>${metRows || '<tr><td colspan="4" class="muted">none</td></tr>'}</tbody>
        </table>
      </div>`;
    }).join("");
  })();

  /* ---------- diagrams tab (lazy mermaid) ---------- */
  let diagramsRendered = false;
  function renderDiagramsOnce() {
    if (diagramsRendered) return;
    diagramsRendered = true;
    const host = $("#diagrams-list");
    const items = [];
    clusters.forEach((c) => (c.diagrams || []).forEach((d) =>
      items.push({ title: `[${c.cluster}] ${d.title}`, mermaid: d.mermaid || "" })));
    host.innerHTML = items.map((d, i) => `
      <div class="diagram">
        <h3>${esc(d.title)}</h3>
        <div class="src">mermaid source</div>
        <div class="mermaid" id="dg-${i}"></div>
        <pre id="dgp-${i}">${esc(d.mermaid)}</pre>
      </div>`).join("");
    const canMermaid = window.mermaid && !window.__mermaidFailed;
    if (canMermaid) {
      try { window.mermaid.initialize({ startOnLoad: false, theme: "dark", securityLevel: "loose" }); } catch (e) { /* noop */ }
      items.forEach((d, i) => {
        const el = document.getElementById("dg-" + i);
        window.mermaid.render("mmt" + i, d.mermaid).then(
          (r) => { el.innerHTML = r.svg; document.getElementById("dgp-" + i).style.display = "none"; },
          () => { el.style.display = "none"; }
        );
      });
    } else {
      items.forEach((d, i) => { document.getElementById("dg-" + i).style.display = "none"; });
    }
  }

  /* ---------- auditors tab ---------- */
  (function auditors() {
    const pageOf = D.page_of || {};
    $("#auditors-grid").innerHTML = allAuditors.map((a) => {
      const n = allFindings.filter((f) => (f.engineer || "").includes(a.num + " ")).length;
      const page = pageOf[a.num];
      const inner = `
        <span class="n">${n} finding${n === 1 ? "" : "s"}</span>
        <div class="num">${esc(a.num)} · ${esc(a._cluster)}</div>
        <div class="name">${esc(a.name)}</div>
        <div class="lens">${esc(a.lens || "")}</div>`;
      return page
        ? `<a class="auditor" href="agents/${esc(page)}.html" style="text-decoration:none;color:inherit;display:block">${inner}</a>`
        : `<div class="auditor">${inner}</div>`;
    }).join("");
  })();
})();
