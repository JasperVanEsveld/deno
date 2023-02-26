use crate::{standalone::Metadata, DENO, WEBVIEW};
use deno_core::{resolve_url_or_path, ModuleResolutionError};
use deno_runtime::colors;
use secure_tempfile::Builder;
use std::{collections::HashMap, thread};
use wry::{
  application::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{
      ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget,
    },
    platform::windows::EventLoopExtWindows,
    window::{Fullscreen, Window, WindowBuilder, WindowId},
  },
  webview::{WebContext, WebView, WebViewBuilder},
};

pub mod ipc;

#[derive(Debug)]
enum UserEvents {
  Message(String),
  CloseWindow(WindowId),
  NewWindow(String, String),
}

#[derive(Debug)]
pub struct WebViewConfig {
  pub default_title: String,
  pub default_url: String,
  pub decorations: Option<bool>,
  pub transparent: Option<bool>,
  pub dev_tools: Option<bool>,
}

pub fn create_webview_config(
  metadata: &Metadata,
) -> Result<WebViewConfig, ModuleResolutionError> {
  let source = match &metadata.webview_url {
    Some(url) => {
      println!("{}", url);
      resolve_url_or_path(url)?
    }
    None => resolve_url_or_path("./index.html")?,
  };
  println!("{}", source.to_string());
  let title = match &metadata.title {
    Some(title) => title.clone(),
    None => "Webview".to_string(),
  };
  Ok(WebViewConfig {
    default_title: title,
    default_url: source.to_string(),
    decorations: Some(metadata.decorations.clone()),
    transparent: Some(metadata.transparent.clone()),
    dev_tools: Some(metadata.dev_tools.clone()),
  })
}

pub fn start_webview(config: WebViewConfig) -> i8 {
  let mut views = HashMap::<WindowId, WebView>::new();
  let event_loop = EventLoop::<UserEvents>::new_any_thread();
  let proxy = event_loop.create_proxy();
  let external_proxy = event_loop.create_proxy();
  let tmp_dir = Builder::new().tempdir().unwrap();
  let path = tmp_dir.path().to_path_buf();
  let window = create_window(
    &config,
    WebContext::new(Some(path.clone())),
    config.default_url.clone(),
    config.default_title.clone(),
    &event_loop,
    proxy.clone(),
  )
  .unwrap();
  views.insert(window.0, window.1);

  pass_messages(external_proxy);

  event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::NewEvents(StartCause::Init) => {
        println!("{}", colors::green("Webview started"))
      }
      Event::WindowEvent {
        event, window_id, ..
      } => match event {
        WindowEvent::CloseRequested => {
          views.remove(&window_id);
          if views.is_empty() {
            *control_flow = ControlFlow::Exit
          }
        }
        WindowEvent::Resized(_) => {
          // let _ = views[&window_id].resize();
        }
        _ => (),
      },
      Event::UserEvent(UserEvents::NewWindow(url, title)) => {
        match create_window(
          &config,
          WebContext::new(Some(path.clone())),
          url,
          title,
          &event_loop,
          proxy.clone(),
        ) {
          Ok((id, view)) => {
            views.insert(id, view);
          }
          _ => {}
        }
      }
      Event::UserEvent(UserEvents::CloseWindow(id)) => {
        views.remove(&id);
        if views.is_empty() {
          *control_flow = ControlFlow::Exit
        }
      }
      Event::UserEvent(UserEvents::Message(message)) => {
        let code = format!("window.deno.triggerMessage(`{message}`)");
        for view in views.iter() {
          view.1.evaluate_script(&code).unwrap();
        }
      }
      _ => (),
    }
  });
}

fn create_window(
  config: &WebViewConfig,
  mut web_context: WebContext,
  url: String,
  title: String,
  event_loop: &EventLoopWindowTarget<UserEvents>,
  proxy: EventLoopProxy<UserEvents>,
) -> wry::Result<(WindowId, WebView)> {
  let borrow_title = &title.clone();
  let borrow_url = &url.clone();

  let handler = move |window: &Window, req: String| {
    if req == "fullscreen" {
      if let Some(_full) = window.fullscreen() {
        window.set_fullscreen(None);
      } else {
        window.set_fullscreen(Some(Fullscreen::Borderless(None)));
      }
    }
    if req == "minimize" {
      window.set_minimized(true);
    }
    if req == "maximize" {
      window.set_maximized(!window.is_maximized());
    }
    if req == "close" {
      let _ = proxy.send_event(UserEvents::CloseWindow(window.id()));
    }
    if req.starts_with("window") {
      let arguments = req.replace("window:", "");
      let split: Vec<&str> = arguments.split(",").collect();
      let (url, title) = match split.len() {
        0 => (url.clone(), title.clone()),
        1 => (split[0].to_string(), title.clone()),
        _ => (split[0].to_string(), split[1].to_string()),
      };
      let _ = proxy.send_event(UserEvents::NewWindow(url, title));
    }
    if req.starts_with("deno:") {
      let message = req.replace("deno:", "");
      thread::spawn(|| {
        let tx = DENO.0.blocking_lock();
        tx.send(message).unwrap();
      });
    }
    if req == "drag_window" {
      let _ = window.drag_window();
    }
  };

  let script = include_str!("webview.js");
  let transparent = config.transparent.unwrap_or(false);
  let window = WindowBuilder::new()
    .with_decorations(config.decorations.unwrap_or(true))
    .with_title(borrow_title)
    .with_transparent(transparent)
    .with_inner_size(LogicalSize::<i16>::new(1680, 840))
    .build(&event_loop)?;
  let window_id = window.id();
  let webview = WebViewBuilder::new(window)?
    .with_url(borrow_url)?
    .with_ipc_handler(handler)
    .with_initialization_script(script)
    .with_transparent(transparent)
    .with_devtools(config.dev_tools.unwrap_or(false))
    .with_web_context(&mut web_context)
    .build()?;
  Ok((window_id, webview))
}

fn pass_messages(proxy: EventLoopProxy<UserEvents>) {
  thread::spawn(move || {
    let mut rx = WEBVIEW.1.blocking_lock();
    while let Some(msg) = rx.blocking_recv() {
      proxy.send_event(UserEvents::Message(msg)).unwrap();
    }
  });
}
