use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3::types::PyFunction;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tao::{
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    window::{Window, WindowBuilder, WindowId},
};
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use wry::{http::Request, WebView, WebViewBuilder};
use wry::cookie::time::macros::date;

fn create_new_window<F>(
    title: String,
    event_loop: &EventLoopWindowTarget<UserEvent>,
    handler: F,
) -> (Window, WebView)
where
    F: Fn(Request<String>) + 'static,
{
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize {
            width: 800,
            height: 600,
        })
        .build(event_loop)
        .unwrap();
    let builder = WebViewBuilder::new().with_ipc_handler(handler).with_url("https://tauri.app");
    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let webview = builder.build(&window).unwrap();
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        builder.build_gtk(vbox).unwrap()
    };
    (window, webview)
}

#[derive(Debug)]
enum UserEvent {
    CloseWindow(WindowId),
    Request(IPCData),
}

#[derive(Debug, Serialize, Deserialize)]
struct IPCData {
    event_type: String,
    request_id: String,
    args: Box<Vec<Value>>,
}


#[pyclass]
struct WindowManager {
    webviews: Arc<Mutex<HashMap<String, WindowId>>>,
    commands: Arc<Mutex<HashMap<String, Py<PyFunction>>>>,
    listeners: Arc<Mutex<HashMap<String, Py<PyFunction>>>>,
}

#[pymethods]
impl WindowManager {
    #[new]
    fn py_new() -> PyResult<Self> {
        Ok(Self {
            webviews: Arc::new(Mutex::new(HashMap::new())),
            commands: Arc::new(Mutex::new(HashMap::new())),
            listeners: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    #[pyo3(text_signature = "(self, name, callback)")]
    fn add_callback(&mut self, name: String, callback: Py<PyFunction>) {
        self.commands.lock().unwrap().insert(name, callback);
    }

    #[pyo3(text_signature = "(self, name, callback)")]
    fn add_listener(&mut self, name: String, callback: Py<PyFunction>) {
        self.listeners.lock().unwrap().insert(name, callback);
    }


    #[pyo3(text_signature = "(self)")]
    fn run(&mut self) {
        let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();
        let commands = self.commands.clone();
        let listener = self.listeners.clone();
        let new_window = create_new_window(
            format!("Window {}", self.webviews.lock().unwrap().len() + 1),
            &event_loop,
            move |req: Request<String>| {
                let data: IPCData = serde_json::from_str(req.body()).unwrap();
                match data.event_type.as_str() {
                    "event" => {
                        proxy.clone().send_event(UserEvent::Request(data)).unwrap();
                    }
                    "request" => {
                        proxy.clone().send_event(UserEvent::Request(data)).unwrap();
                    }
                    _ => ()
                }
            },
        );
        self.webviews.lock().unwrap().insert("main".to_string(), new_window.0.id());
        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(StartCause::Init) => {
                    println!("Webview started ...")
                }
                Event::UserEvent(UserEvent::Request(req)) => {}
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => ()
            }
        });
    }
}


/// A Python module implemented in Rust.
#[pymodule]
#[pyo3(name = "pywui_rs")]
fn pywui_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<WindowManager>();
    Ok(())
}
