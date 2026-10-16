// Applies a saved light/dark choice before first paint (loaded blocking in <head>).
try {
  const t = localStorage.getItem("theme");
  if (t === "light" || t === "dark") document.documentElement.dataset.theme = t;
} catch {}
