// Kimi Code Web only: add a Chinese "open in new window" action to the
// sidebar session context menu. Prefer the session data already attached to
// the right-clicked Vue component; borrow the native copy action only as a
// compatibility fallback, without reading or changing the system clipboard.
document.addEventListener("DOMContentLoaded", () => {
  if (
    !window.pakeConfig ||
    !/^https?:\/\/127\.0\.0\.1/.test(window.pakeConfig.url || "")
  )
    return;
  if (!window.__TAURI__ || !window.__TAURI__.core) return;

  // macOS overlay title-bar traffic lights overlap the Kimi sidebar logo/header.
  // Push the header right and hide the in-page brand, matching Kimi's own
  // .macos-desktop rules without entering the full desktop mode that requires
  // window.kimiDesktop APIs Pake does not provide.
  const isMac = /Mac/i.test(navigator.platform || navigator.userAgent);
  if (isMac && window.pakeConfig.hide_title_bar === true) {
    const style = document.createElement("style");
    style.textContent = `
      .side .ch {
        padding-left: 80px !important;
        -webkit-app-region: drag;
      }
      .side .ch-brand {
        display: none !important;
      }
    `;
    document.head.appendChild(style);
  }

  const LABEL = "在新窗口中打开";
  const COPY_LABEL = /^(Copy Session ID|复制 Session ID)/;
  const SESSION_ID = /^session_[0-9a-f-]+$/i;
  const ICON =
    "M10 6v2H5v11h11v-5h2v6a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1zm11-3v8h-2V6.413l-7.793 7.794l-1.414-1.414L17.585 5H13V3z";
  let contextTarget = null;

  document.addEventListener(
    "contextmenu",
    (event) => {
      contextTarget = event.target instanceof Element ? event.target : null;
    },
    true,
  );

  function makeItem(copyItem) {
    const item = copyItem.cloneNode(true);
    item.classList.add("pake-open-session");
    item.removeAttribute("data-state");

    const svg = item.querySelector("svg");
    if (svg) {
      svg.setAttribute("viewBox", "0 0 24 24");
      svg.innerHTML = '<path fill="currentColor" d="' + ICON + '"/>';
    }

    const walker = document.createTreeWalker(item, NodeFilter.SHOW_TEXT);
    let textNode = walker.nextNode();
    while (
      textNode &&
      !COPY_LABEL.test((textNode.nodeValue || "").replace(/\s+/g, " ").trim())
    ) {
      textNode = walker.nextNode();
    }
    if (textNode) textNode.nodeValue = LABEL;
    else item.append(document.createTextNode(LABEL));
    return item;
  }

  function findCopyItem(menu) {
    for (const node of menu.querySelectorAll(
      "[role=button], button, [role=menuitem], div",
    )) {
      const label = (node.textContent || "").replace(/\s+/g, " ").trim();
      if (COPY_LABEL.test(label)) return node;
    }
    return null;
  }

  function sessionIdFromContextTarget() {
    for (
      let element = contextTarget;
      element;
      element = element.parentElement
    ) {
      for (
        let component = element.__vueParentComponent;
        component;
        component = component.parent
      ) {
        const candidates = [
          component.props?.session?.id,
          component.vnode?.props?.session?.id,
          component.setupState?.session?.id,
        ];
        const id = candidates.find(
          (value) => typeof value === "string" && SESSION_ID.test(value),
        );
        if (id) return id;
      }
    }
    return "";
  }

  async function sessionIdFromCopyAction(copyItem) {
    const clipboard = navigator.clipboard;
    if (!clipboard || typeof clipboard.writeText !== "function") return "";

    const ownDescriptor = Object.getOwnPropertyDescriptor(
      clipboard,
      "writeText",
    );
    let captured = "";
    try {
      Object.defineProperty(clipboard, "writeText", {
        configurable: true,
        value: async (text) => {
          if (typeof text === "string" && SESSION_ID.test(text.trim()))
            captured = text.trim();
        },
      });
      copyItem.click();
      await new Promise((resolve) => setTimeout(resolve, 50));
      return captured;
    } finally {
      if (ownDescriptor)
        Object.defineProperty(clipboard, "writeText", ownDescriptor);
      else delete clipboard.writeText;
    }
  }

  async function openSession(copyItem) {
    const id =
      sessionIdFromContextTarget() || (await sessionIdFromCopyAction(copyItem));
    if (!id) {
      if (window.pakeToast) window.pakeToast("无法获取会话 ID");
      return;
    }
    // Carry the current auth token fragment so the new window can load the
    // session without showing the auth gate.
    const hash = window.location.hash || "";
    const url =
      window.location.origin + "/sessions/" + encodeURIComponent(id) + hash;
    try {
      await window.__TAURI__.core.invoke("open_session_window", { url });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (window.pakeToast) window.pakeToast("打开新窗口失败: " + message);
      console.error("[Pake] open_session_window failed:", error);
    }
  }

  const observer = new MutationObserver(() => {
    for (const menu of document.querySelectorAll(".menu")) {
      if (menu.querySelector(".pake-open-session")) continue;
      const copyItem = findCopyItem(menu);
      if (!copyItem) continue;
      const item = makeItem(copyItem);
      item.addEventListener("click", (event) => {
        event.preventDefault();
        event.stopPropagation();
        openSession(copyItem);
      });
      copyItem.insertAdjacentElement("afterend", item);
    }
  });
  observer.observe(document.body, { childList: true, subtree: true });
});
