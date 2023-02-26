use deno_core::op;
use deno_core::Extension;
use deno_core::ExtensionFileSource;
use deno_core::ExtensionFileSourceCode;
use deno_core::OpState;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{DENO, WEBVIEW};

#[op]
async fn op_ipc_recv() -> String {
  let mut rx = DENO.1.lock().await;
  rx.recv().await.unwrap()
}

#[op]
async fn op_ipc_send(_state: Rc<RefCell<OpState>>, value: String) {
  let tx = WEBVIEW.0.lock().await;
  tx.send(value).unwrap();
}

pub fn webview_extension() -> Extension {
  Extension::builder("webview")
    .ops(vec![op_ipc_send::decl(), op_ipc_recv::decl()])
    .js(vec![ExtensionFileSource {
      specifier: "ipc.js".to_string(),
      code: ExtensionFileSourceCode::IncludedInBinary(include_str!("ipc.js")),
    }])
    .build()
}
