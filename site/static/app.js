// Sidebar tabs that swap the screenshot, a light/dark switch, live graphs and menu-bar stats like
// the app's, and copy buttons. Kept in a file (not inline) because the CSP allows only same-origin scripts.

const root = document.documentElement;
const darkQuery = matchMedia("(prefers-color-scheme: dark)");
const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
const tabs = [...document.querySelectorAll('[role="tab"]')];
const shot = document.getElementById("shot");
const caption = document.getElementById("caption");
let current = tabs[0];

const theme = () => root.dataset.theme || (darkQuery.matches ? "dark" : "light");

// Crossfades to `src`; the fade-in happens once the new image has loaded.
function show(src) {
  if (shot.getAttribute("src") === src) return;
  if (still) {
    shot.src = src;
    return;
  }
  shot.classList.add("out");
  setTimeout(() => (shot.src = src), 160);
}
shot.addEventListener("load", () => shot.classList.remove("out"));
shot.addEventListener("error", () => shot.classList.remove("out"));

function render() {
  show(`/screenshots/${theme()}-${current.dataset.shot}.png`);
  for (const b of document.querySelectorAll("[data-set-theme]")) {
    b.setAttribute("aria-pressed", String(b.dataset.setTheme === theme()));
  }
}

function select(tab, focus) {
  current = tab;
  for (const t of tabs) {
    t.setAttribute("aria-selected", String(t === tab));
    t.tabIndex = t === tab ? 0 : -1;
  }
  shot.alt = `The ${tab.textContent.trim()} page of Resources`;
  caption.textContent = tab.dataset.text;
  render();
  if (focus) tab.focus();
}

tabs.forEach((tab, i) => {
  tab.tabIndex = i === 0 ? 0 : -1;
  tab.addEventListener("click", () => select(tab, false));
  // Arrow keys move between tabs, as the WAI-ARIA tabs pattern expects.
  tab.addEventListener("keydown", (e) => {
    const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key];
    if (!step) return;
    e.preventDefault();
    select(tabs[(i + step + tabs.length) % tabs.length], true);
  });
});

for (const b of document.querySelectorAll("[data-set-theme]")) {
  b.addEventListener("click", () => {
    root.dataset.theme = b.dataset.setTheme;
    try {
      localStorage.setItem("theme", b.dataset.setTheme);
    } catch {}
    render();
  });
}
darkQuery.addEventListener("change", render);
render();

// Warm the cache so switching tabs doesn't flash.
addEventListener("load", () => {
  for (const t of tabs) new Image().src = `/screenshots/${theme()}-${t.dataset.shot}.png`;
});

// Sidebar graphs: tinted background, 40% area fill, full-colour line (the app's ResGraph look).
const W = 100, H = 30, N = 30;
const ns = "http://www.w3.org/2000/svg";
const el = (name, attrs) => {
  const e = document.createElementNS(ns, name);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
};
const walk = (v) => Math.min(0.92, Math.max(0.06, v + (Math.random() - 0.5) * 0.18));

const graphs = tabs.filter((t) => t.dataset.graph).map((tab) => {
  const color = getComputedStyle(root).getPropertyValue(`--${tab.dataset.graph}`).trim();
  const svg = el("svg", { class: "graph", viewBox: `0 0 ${W} ${H}`, preserveAspectRatio: "none", "aria-hidden": "true" });
  const area = el("path", { fill: color, "fill-opacity": "0.4" });
  const line = el("path", { fill: "none", stroke: color, "stroke-width": "1.5", "vector-effect": "non-scaling-stroke" });
  svg.append(el("rect", { width: W, height: H, fill: color, "fill-opacity": "0.12" }), area, line);
  tab.append(svg);
  let v = 0.2 + Math.random() * 0.5;
  return { name: tab.dataset.graph, area, line, values: Array.from({ length: N }, () => (v = walk(v))) };
});

function draw() {
  for (const g of graphs) {
    const pts = g.values.map((v, i) => `${((i / (N - 1)) * W).toFixed(1)},${(H - v * H).toFixed(1)}`);
    g.line.setAttribute("d", `M${pts.join("L")}`);
    g.area.setAttribute("d", `M0,${H}L${pts.join("L")}L${W},${H}Z`);
  }
}

// Menu-bar stats follow the Processor and Memory graphs.
const cpuText = document.getElementById("mb-cpu");
const memText = document.getElementById("mb-mem");
const last = (name) => `${Math.round(graphs.find((g) => g.name === name).values[N - 1] * 100)}%`;

function tick() {
  draw();
  cpuText.textContent = last("cpu");
  memText.textContent = last("memory");
}

tick();
if (!still) {
  let n = 0;
  setInterval(() => {
    for (const g of graphs) {
      g.values.shift();
      g.values.push(walk(g.values[N - 2]));
    }
    draw();
    if (++n % 2 === 0) {
      cpuText.textContent = last("cpu");
      memText.textContent = last("memory");
    }
  }, 1000);
}

for (const btn of document.querySelectorAll("[data-copy]")) {
  const icon = btn.querySelector("use");
  const label = btn.querySelector("span");
  btn.addEventListener("click", async () => {
    const text = document.getElementById(btn.dataset.copy).textContent;
    try {
      await navigator.clipboard.writeText(text);
      icon.setAttribute("href", "#i-check");
      label.textContent = "Copied";
      btn.classList.add("done");
    } catch {
      label.textContent = "Select and copy";
    }
    setTimeout(() => {
      icon.setAttribute("href", "#i-copy");
      label.textContent = "Copy";
      btn.classList.remove("done");
    }, 2000);
  });
}
