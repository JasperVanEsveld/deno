{
  // Set webview functions
  window.webview = {
    isFullscreen: false,
    create: (url, title) => {
      window.ipc.postMessage(`window:${url},${title}`);
    },
    fullscreen: () => {
      window.webview.isFullscreen = !window.webview.isFullscreen;
      window.ipc.postMessage("fullscreen");
      globalThis.dispatchEvent(new Event("fullscreen"));
    },
    minimize: () => {
      window.ipc.postMessage("minimize");
    },
    maximize: () => {
      window.ipc.postMessage("maximize");
    },
    close: () => {
      window.ipc.postMessage("close");
    },
  };

  // Set deno functions
  listeners = [];
  window.deno = {
    triggerMessage: (message) => {
      listeners.forEach((listener) => {
        listener(message);
      });
    },
    onMessage: (listener) => {
      listeners.push(listener);
      return () => {
        const i = listeners.indexOf(listener);
        if (i < 0) {
          return false;
        }
        listeners.splice(i, 1);
        return true;
      };
    },
    send: (message) => {
      window.ipc.postMessage(`deno:${message}`);
    },
  };

  // Listen to drag-region events
  document.addEventListener("mousedown", (e) => {
    if (e.target.classList.contains("drag-region") && e.buttons === 1) {
      e.detail === 2
        ? window.webview.maximize()
        : window.ipc.postMessage("drag_window");
    }
  });
  document.addEventListener("touchstart", (e) => {
    if (
      e.target.classList.contains("drag-region") &&
      window.webview.isFullscreen == false
    ) {
      window.ipc.postMessage("drag_window");
    }
  });
}
