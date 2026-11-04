// 页面动态数据回填：向本站 /api/release 取最新版本数据，失败则保留静态底稿，不报错、不留空。
(async () => {
  try {
    const r = await fetch("/api/release", { headers: { Accept: "application/json" } });
    if (!r.ok) return;
    const d = await r.json();
    const lang = document.documentElement.lang === "en" ? "en" : "zh";
    const fmtDate = (iso) => {
      if (!iso) return null;
      if (lang === "zh") return iso;
      const dt = new Date(iso + "T00:00:00");
      return isNaN(dt) ? iso : dt.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
    };
    if (d.version) document.querySelectorAll('[data-rel="version"]').forEach((el) => (el.textContent = d.version));
    const fd = fmtDate(d.date_iso);
    if (fd) document.querySelectorAll('[data-rel="date"]').forEach((el) => (el.textContent = fd));
    if (d.assets) {
      document.querySelectorAll("[data-asset]").forEach((item) => {
        const a = d.assets[item.dataset.asset];
        if (!a) return;
        const fn = item.querySelector("[data-fname]");
        if (fn) fn.textContent = a.name;
        const fs = item.querySelector("[data-fsize]");
        if (fs && typeof a.size_mb === "number") fs.textContent = a.size_mb.toFixed(1) + " MB";
      });
    }
    const wrap = document.querySelector("[data-changelog]");
    if (wrap && Array.isArray(d.changelog)) {
      const have = new Set([...wrap.querySelectorAll("[data-ver]")].map((e) => e.dataset.ver));
      const missing = d.changelog.filter((s) => !have.has(s.version));
      if (missing.length) {
        if (lang === "en") {
          const note = document.createElement("p");
          note.className = "lede";
          note.style.textAlign = "left";
          note.textContent = "The newest entries below are shown in the original Chinese — English translation pending.";
          wrap.prepend(note);
        }
        [...missing].reverse().forEach((s) => {
          const div = document.createElement("div");
          div.className = "rel";
          div.dataset.ver = s.version;
          let h = `<h3>${s.version}</h3><p class="date">${fmtDate(s.date_iso) || s.date_iso}</p>`;
          for (const g of s.groups || []) {
            h += `<h4>${g.title}</h4><ul>${(g.items || []).map((i) => `<li>${i}</li>`).join("")}</ul>`;
          }
          div.innerHTML = h;
          wrap.prepend(div);
        });
      }
    }
  } catch (e) {
    /* 静态底稿即兜底 */
  }
})();
