document.addEventListener("DOMContentLoaded", () => {
  const debounce = (func, wait) => {
    let timeout;
    return (...args) => {
      clearTimeout(timeout);
      timeout = setTimeout(() => func(...args), wait);
    };
  };

    const updateTheme = () => {
      const doc = document.documentElement;
      const body = document.body;
      let mode = null;

    // Check for explicit theme classes or attributes
    const isDark =
      doc.classList.contains("dark") ||
      body.classList.contains("dark") ||
      doc.getAttribute("data-theme") === "dark" ||
      body.getAttribute("data-theme") === "dark" ||
      doc.style.colorScheme === "dark";

    const isLight =
      doc.classList.contains("light") ||
      body.classList.contains("light") ||
      doc.getAttribute("data-theme") === "light" ||
      body.getAttribute("data-theme") === "light" ||
      doc.style.colorScheme === "light";

    if (isDark) mode = "dark";
    else if (isLight) mode = "light";
    else {
      // Sites that persist the scheme in a data-color-scheme attribute
      // (e.g. Kimi Code Web). "system" resolves to the OS preference so
      // the native title bar follows both the site and the OS.
      const scheme =
        doc.getAttribute("data-color-scheme") ||
        body.getAttribute("data-color-scheme");
      if (scheme === "dark") mode = "dark";
      else if (scheme === "light") mode = "light";
      else if (scheme === "system") {
        mode = window.matchMedia("(prefers-color-scheme: dark)").matches
          ? "dark"
          : "light";
      }
    }

    // Prefer the page's sidebar surface when it exposes one. Kimi defines
    // --color-sidebar-bg as #f9fbfc in light mode and #0d0d0d in dark mode,
    // so the native caption can continue the sidebar instead of the brighter
    // conversation canvas. Other sites fall back to their theme-color meta.
    let color = getComputedStyle(doc).getPropertyValue("--color-sidebar-bg").trim() || null;
    if (!color) {
      for (const meta of document.querySelectorAll('meta[name="theme-color"]')) {
        const media = meta.getAttribute("media");
        const matchesResolvedMode =
          !media ||
          (mode === "dark" && media.includes("prefers-color-scheme: dark")) ||
          (mode === "light" && media.includes("prefers-color-scheme: light"));
        if (matchesResolvedMode) {
          color = meta.getAttribute("content");
          if (media) break;
        }
      }
    }

    // Only invoke Rust command if an explicit theme override is detected
    if (mode && window.__TAURI__?.core) {
      window.__TAURI__.core.invoke("update_theme_mode", { mode, color });
    }
  };

  const debouncedUpdateTheme = debounce(updateTheme, 200);

  // Initial check with delay to allow site to render
  setTimeout(updateTheme, 500);

  // Watch for DOM changes
  const observer = new MutationObserver(debouncedUpdateTheme);
  const config = {
    attributes: true,
    attributeFilter: ["class", "data-theme", "data-color-scheme", "style"],
    subtree: false,
  };

  observer.observe(document.documentElement, config);
  observer.observe(document.body, config);

  // Watch for system theme changes (though window should handle this natively now)
  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", updateTheme);
});
