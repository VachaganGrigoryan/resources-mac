// Sidebar tabs that swap the screenshot, small live graphs like the app's sidebar, and the copy button.
// Kept in a file (not inline) because the server's CSP allows only same-origin scripts.

const tabs = [...document.querySelectorAll('[role="tab"]')];
const shot = document.getElementById("shot");
const shotDark = document.getElementById("shot-dark");
const caption = document.getElementById("caption");

function select(tab, focus) {
  for (const t of tabs) {
    t.setAttribute("aria-selected", String(t === tab));
    t.tabIndex = t === tab ? 0 : -1;
  }
  const name = tab.dataset.shot;
  shot.src = `/screenshots/light-${name}.png`;
  shotDark.srcset = `/screenshots/dark-${name}.png`;
  shot.alt = `The ${tab.textContent.trim()} page of Resources`;
  caption.textContent = tab.dataset.text;
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

// Warm the cache so switching tabs doesn't flash.
const dark = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
addEventListener("load", () => {
  for (const t of tabs) new Image().src = `/screenshots/${dark}-${t.dataset.shot}.png`;
});

// Sidebar graphs: tinted background, 40% area fill, full-colour line (the app's ResGraph look).
const W = 64, H = 28, N = 24;
const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
const graphs = tabs.filter((t) => t.dataset.graph).map((tab) => {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  svg.setAttribute("aria-hidden", "true");
  const color = getComputedStyle(document.documentElement).getPropertyValue(`--${tab.dataset.graph}`).trim();
  const bg = document.createElementNS(ns, "rect");
  bg.setAttribute("width", W);
  bg.setAttribute("height", H);
  bg.setAttribute("fill", color);
  bg.setAttribute("fill-opacity", "0.12");
  const area = document.createElementNS(ns, "path");
  area.setAttribute("fill", color);
  area.setAttribute("fill-opacity", "0.4");
  const line = document.createElementNS(ns, "path");
  line.setAttribute("fill", "none");
  line.setAttribute("stroke", color);
  line.setAttribute("stroke-width", "1.5");
  svg.append(bg, area, line);
  tab.append(svg);
  let v = 0.2 + Math.random() * 0.5;
  const values = Array.from({ length: N }, () => (v = walk(v)));
  return { area, line, values };
});

function walk(v) {
  return Math.min(0.92, Math.max(0.06, v + (Math.random() - 0.5) * 0.18));
}

function draw() {
  for (const g of graphs) {
    const pts = g.values.map((v, i) => `${((i / (N - 1)) * W).toFixed(1)},${(H - v * H).toFixed(1)}`);
    g.line.setAttribute("d", `M${pts.join("L")}`);
    g.area.setAttribute("d", `M0,${H}L${pts.join("L")}L${W},${H}Z`);
  }
}

draw();
if (!still) {
  setInterval(() => {
    for (const g of graphs) {
      g.values.shift();
      g.values.push(walk(g.values[g.values.length - 1]));
    }
    draw();
  }, 1000);
}

for (const btn of document.querySelectorAll("[data-copy]")) {
  btn.addEventListener("click", async () => {
    const text = document.getElementById(btn.dataset.copy).textContent;
    try {
      await navigator.clipboard.writeText(text);
      btn.textContent = "Copied";
    } catch {
      btn.textContent = "Select and copy";
    }
    setTimeout(() => (btn.textContent = "Copy"), 2000);
  });
}
