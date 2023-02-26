// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
"use strict";

{
  const core = Deno.core;
  const listeners = [];
  const messages = {
    [Symbol.asyncIterator]() {
      return {
        async next() {
          return {
            value: await core.opAsync("op_ipc_recv"),
            done: false,
          };
        },
      };
    },
  };

  async function listenForMessages() {
    for await (const message of messages) {
      listeners.forEach((listener) => listener(message));
    }
  }
  listenForMessages();

  const webview = {
    send: (message) => {
      core.opAsync("op_ipc_send", message);
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
  };

  globalThis.webview = webview;
}
