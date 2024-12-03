use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use pyo3::prelude::*;
use pyo3::types::PyFunction;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tao::{event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget}, window::{Window, WindowBuilder, WindowId}};
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use wry::{http::Request, WebView, WebViewBuilder};
use wry::WebViewAttributes;

use crate::config::Config;
use crate::util::{json_to_py, load_config, py_to_json};

mod config;
mod util;
mod webview;
mod window;

fn create_new_window(
    title: String,
    webview: WebViewAttributes,
    event_loop: &EventLoopWindowTarget<UserEvent>,
) -> (Window, WebView) {
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize {
            width: 800,
            height: 600,
        })
        .build(event_loop)
        .unwrap();
    let builder = WebViewBuilder::with_attributes(webview);
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window).unwrap();
    #[cfg(target_os = "linux")]
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
    Response(ResponseData),
    Emit(EmitData),
}

#[derive(Serialize, Deserialize)]
struct IPCData {
    event_type: String,
    request_id: String,
    args: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseData {
    request_id: String,
    data: Box<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmitData {
    event: String,
    data: Box<Value>,
}

#[pyclass]
struct WindowManager {
    webviews: Arc<Mutex<HashMap<String, WindowId>>>,
    commands: Arc<Mutex<HashMap<String, Py<PyFunction>>>>,
    listeners: Arc<Mutex<HashMap<String, Py<PyFunction>>>>,
    config: Arc<Mutex<Config>>,
}

#[pymethods]
impl WindowManager {
    #[new]
    #[pyo3(text_signature = "(config_path)")]
    fn py_new(config_path: String) -> PyResult<Self> {
        Ok(Self {
            webviews: Arc::new(Mutex::new(HashMap::new())),
            commands: Arc::new(Mutex::new(HashMap::new())),
            listeners: Arc::new(Mutex::new(HashMap::new())),
            config: Arc::new(Mutex::new(load_config(config_path.as_str()).unwrap())),
        })
    }

    #[pyo3(text_signature = "(self, name, callback)")]
    fn add_command(&mut self, name: String, callback: Py<PyFunction>) {
        self.commands.lock().unwrap().insert(name, callback);
    }

    #[pyo3(text_signature = "(self, name, callback)")]
    fn add_listener(&mut self, name: String, callback: Py<PyFunction>) {
        self.listeners.lock().unwrap().insert(name, callback);
    }

    #[pyo3(text_signature = "(self, event, data)")]
    fn emit(&mut self, event: String, data: PyObject) {
        println!("Hello")
    }

    #[pyo3(text_signature = "(self)")]
    fn run(&mut self) {
        let mut webview_windows: HashMap<WindowId, (Window, WebView)> = HashMap::new();
        let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();
        let command = self.commands.clone();
        let listener = self.listeners.clone();
        let config = self.config.lock().unwrap();

        // Wrap listeners and commands inside Arc<Mutex> to allow shared ownership.
        let listeners = listener.clone();
        let commands = command.clone();

        let handler: Arc<Mutex<Box<dyn Fn(Request<String>)>>> = Arc::new(Mutex::new(Box::new(move |req: Request<String>| {
            let data: IPCData = serde_json::from_str(req.body()).unwrap();
            let listeners = listeners.lock().unwrap();
            let commands = commands.lock().unwrap();
            match data.event_type.as_str() {
                "event" => {
                    if let Some(func) = listeners.get(data.event_type.as_str()) {
                        Python::with_gil(|py| {
                            let args: PyObject = json_to_py(py, &data.args);
                            let py_args = args.downcast_bound::<pyo3::types::PyTuple>(py).unwrap();
                            func.call1(py, py_args).unwrap();
                        });
                    }
                }
                "request" => {
                    if let Some(func) = commands.get(data.event_type.as_str()) {
                        Python::with_gil(|py| {
                            let args: PyObject = json_to_py(py, &data.args);
                            let py_args = args.downcast_bound::<pyo3::types::PyTuple>(py).unwrap();
                            let value = func.call1(py, py_args).unwrap();
                            proxy.clone().send_event(UserEvent::Response(ResponseData {
                                request_id: data.request_id,
                                data: Box::new(py_to_json(py, value)),
                            })).unwrap();
                        });
                    }
                }
                _ => {}
            }
        })));

        for win in config.pywui.windows.iter().clone() {
            let default_value = if let Some(web_conf) = win.webview.clone() {
                WebViewAttributes::from(web_conf)
            } else {
                WebViewAttributes::default()
            };
            // Create a handler closure that captures listeners and commands in Arc<Mutex>
            let cloned_handler = handler.clone();
            let web_view = WebViewAttributes {
                url: Option::from(default_value.url.unwrap_or(config.clone().build.dev_path)),
                initialization_scripts: vec![],
                ipc_handler: Some(Box::new( move |req: Request<String>| {
                    let handler_lock =  cloned_handler.lock().unwrap();
                    handler_lock(req)
                })),
                ..default_value
            };

            let new_window = create_new_window(
                format!("Window {}", self.webviews.lock().unwrap().len() + 1),
                web_view,
                &event_loop,
            );
            let window_id = new_window.0.id();
            webview_windows.insert(window_id.clone(), new_window);
            self.webviews.lock().unwrap().insert("main".to_string(), window_id.clone());
        }

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(StartCause::Init) => {
                    println!("Webview started ...")
                }
                Event::UserEvent(UserEvent::Response(data)) => {
                    for (_, webview) in webview_windows.iter().clone() {
                        let js_code = format!(
                            r#"
                                const event = new CustomEvent('{}', {{
                                    detail: {}
                                }});
                                window.dispatchEvent(event);
                            "#,
                            data.request_id, data.data
                        );
                        webview.1.evaluate_script(js_code.as_str()).unwrap();
                    }
                    println!("String {}", webview_windows.len().to_string())
                }
                Event::UserEvent(UserEvent::Emit(data)) => {
                    for (_, webview) in webview_windows.iter().clone() {
                        let js_code = format!(
                            r#"
                                const event = new CustomEvent('{}', {{
                                    detail: {}
                                }});
                                window.dispatchEvent(event);
                            "#,
                            data.event, data.data
                        );
                        webview.1.evaluate_script(js_code.as_str()).unwrap();
                    }
                    println!("String {}", webview_windows.len().to_string())
                }
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
    let _ = m.add_class::<WindowManager>();
    Ok(())
}
