/* Agent page helper: render mermaid sources embedded as <script type="text/plain" class="mermaid-src"> */
(function () {
  function init() {
    var sources = Array.prototype.slice.call(document.querySelectorAll('script.mermaid-src'));
    if (!sources.length) return;
    var render = function () {
      if (!window.mermaid) return; // fallback <pre> blocks remain visible
      try { window.mermaid.initialize({ startOnLoad: false, theme: "dark", securityLevel: "loose" }); } catch (e) { return; }
      sources.forEach(function (s, i) {
        var box = document.getElementById(s.dataset.target);
        if (!box) return;
        window.mermaid.render("am" + i, s.textContent).then(
          function (r) { box.innerHTML = r.svg; var fb = box.querySelector("pre"); if (fb) fb.style.display = "none"; },
          function () { /* keep <pre> fallback */ }
        );
      });
    };
    if (window.mermaid && !window.__mermaidFailed) render();
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
  else init();
})();
