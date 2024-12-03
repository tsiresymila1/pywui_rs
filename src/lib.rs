use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use http::header::CONTENT_TYPE;
use http::Response;

use image::EncodableLayout;
use pyo3::prelude::*;
use pyo3::types::PyFunction;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tao::{event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget}, window::{Window, WindowBuilder, WindowId}};
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::EventLoopProxy;
use tao::window::Icon;
use wry::{http::Request, RequestAsyncResponder, WebView, WebViewBuilder, WebViewExtMacOS, WebViewId};
use wry::WebViewAttributes;

use crate::config::Config;
use crate::util::{json_to_py, load_config, py_to_json};
use crate::window::WindowAttributesConfig;

mod config;
mod util;
mod webview;
mod window;

fn get_wry_response(
    request: Request<Vec<u8>>,
    responder: RequestAsyncResponder,
) {
    let path = request.uri().path();
    // Read the file content from file path
    let root = PathBuf::from("examples/custom_protocol");
    let path = if path == "/" {
        "index.html"
    } else {
        &path[1..]
    };
    let content = fs::read(fs::canonicalize(root.join(path)).unwrap()).unwrap();
    let mimetype = if path.ends_with(".html") || path == "/" {
        "text/html"
    } else if path.ends_with(".js") {
        "text/javascript"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".wasm") {
        "application/wasm"
    } else {
        responder.respond(Response::builder()
            .header(CONTENT_TYPE, "text/plain")
            .status(500)
            .body("Not found".to_string().as_bytes().to_vec())
            .unwrap());
        return;
    };
    responder.respond(Response::builder()
        .header(CONTENT_TYPE, mimetype)
        .body(content)
        .unwrap())
}

fn create_new_window(
    title: String,
    webview: WebViewAttributes,
    window: WindowAttributesConfig,
    event_loop: &EventLoopWindowTarget<UserEvent>,
    icon: Option<String>,
) -> (Window, WebView) {
    println!("Image path {}", icon.clone().unwrap());
    let icon = {
        let bytes = match fs::read(format!("{}", icon.unwrap_or("pywui.png".to_string()).to_string())) {
            Ok(data) => data,
            Err(e) => panic!("Failed to read the file: {}", e),
        };
        let image = match image::load_from_memory(bytes.as_bytes()) {
            Ok(img) => img.into_rgba8(),
            Err(e) => panic!("Failed to decode image: {}", e),
        };
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        Icon::from_rgba(rgba, width, height).unwrap()
    };
    let window = WindowBuilder::new()
        .with_title(window.title.unwrap_or("Window".to_string()))
        .with_inner_size(LogicalSize {
            width: window.width.unwrap_or(800),
            height: window.height.unwrap_or(600),
        })
        .with_decorations(window.decorations.unwrap_or(true))
        .with_transparent(window.transparent.unwrap_or(false))
        .with_background_color(window.background_color.unwrap_or((255, 255, 255, 0)))
        .with_always_on_top(window.always_on_top.unwrap_or(false))
        .with_closable(window.closable.unwrap_or(true))
        .with_maximized(window.maximized.unwrap_or(false))
        .with_maximizable(window.maximizable.unwrap_or(true))
        .with_minimizable(window.minimizable.unwrap_or(true))
        .with_focused(window.focused.unwrap_or(true))
        .with_resizable(window.resizable.unwrap_or(true))
        .with_visible(window.visible.unwrap_or(true))
        .with_window_icon(Some(icon))
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
    Exit(WindowId),
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
    proxy: Option<Arc<Mutex<EventLoopProxy<UserEvent>>>>,
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
            proxy: None,
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
    fn emit(&self, event: String, data: PyObject) {
        if let Some(proxy) = self.proxy.clone() {
            Python::with_gil(|py| {
                proxy.lock().unwrap().send_event(UserEvent::Emit(EmitData {
                    event,
                    data: Box::new(py_to_json(py, data)),
                })).unwrap();
            })
        }
        // webview.evaluate_script().unwrap()
    }
    #[pyo3(text_signature = "(self,label= None)")]
    fn exit(&self, label: Option<String>) {
        if let Some(proxy) = self.proxy.clone() {
            if let Some(window_id) = self.webviews.clone().lock().unwrap().get(&label.unwrap().clone()) {
                Python::with_gil(|py| {
                    proxy.lock().unwrap().send_event(UserEvent::Exit(*window_id)).unwrap();
                })
            }
        }
        // webview.evaluate_script().unwrap()
    }

    #[pyo3(text_signature = "(self)")]
    fn run(&mut self) {
        let mut webview_windows: HashMap<WindowId, (Window, WebView)> = HashMap::new();
        let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();
        self.proxy = Some(Arc::new(Mutex::new(proxy.clone())));
        let command = self.commands.clone();
        let listener = self.listeners.clone();
        let config = self.config.lock().unwrap();

        // Wrap listeners and commands inside Arc<Mutex> to allow shared ownership.
        let listeners = listener.clone();
        let commands = command.clone();

        let protocol_handler: Arc<Mutex<Box<dyn Fn(WebViewId, Request<Vec<u8>>, RequestAsyncResponder)>>> = Arc::new(Mutex::new(Box::new(move |id, request, responder| {
            get_wry_response(request, responder)
        })));

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
            let proto_handler = protocol_handler.clone();

            let custom_protocols: HashMap<String, Box<dyn Fn(WebViewId, Request<Vec<u8>>, RequestAsyncResponder)>> =
                HashMap::from([(
                    "pywui".to_string(),
                    Box::new(move |w: WebViewId, req: Request<Vec<u8>>, res: RequestAsyncResponder| {
                        let handler_lock = proto_handler.lock().unwrap();
                        handler_lock(w, req, res)
                    }) as Box<dyn Fn(WebViewId, Request<Vec<u8>>, RequestAsyncResponder)>
                )]);

            let web_view = WebViewAttributes {
                url: Option::from(default_value.url.unwrap_or(config.clone().build.dev_path)),
                initialization_scripts: vec![],
                ipc_handler: Some(Box::new(move |req: Request<String>| {
                    let handler_lock = cloned_handler.lock().unwrap();
                    handler_lock(req)
                })),
                custom_protocols,
                ..default_value
            };

            let new_window = create_new_window(
                format!("Window {}", self.webviews.lock().unwrap().len() + 1),
                web_view,
                win.clone(),
                &event_loop,
                config.clone().icon.get_for_current_os(),
            );
            let window_id = new_window.0.id();
            webview_windows.insert(window_id.clone(), new_window);
            self.webviews.lock().unwrap().insert("main".to_string(), window_id.clone());
        }

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(StartCause::Init) => {
                    println!("Pywui started ...")
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
                }
                Event::UserEvent(
                    UserEvent::Exit(window_id)
                ) => {
                    webview_windows.remove(&window_id);
                    if webview_windows.len() == 0{
                        println!("Pywui exit ....");
                        *control_flow = ControlFlow::Exit
                    }
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    webview_windows.remove(&window_id);
                    if webview_windows.len() == 0 {
                        println!("Pywui exit ....");
                        *control_flow = ControlFlow::Exit
                    }
                }
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
