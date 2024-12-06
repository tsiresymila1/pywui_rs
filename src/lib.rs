use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::os::unix::raw::mode_t;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use http::header::CONTENT_TYPE;
use http::Response;
use image::EncodableLayout;
use pyo3::prelude::*;
use pyo3::types::{PyFunction, PyTuple};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tao::{
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget},
    window::{Window, WindowBuilder, WindowId},
};
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{EventLoop, EventLoopProxy};
use tao::window::Icon;
use wry::{
    http::Request, RequestAsyncResponder, WebView, WebViewBuilder, WebViewExtMacOS, WebViewId,
};
use wry::WebViewAttributes;

use crate::config::Config;
use crate::init_script::get_init_script;
use crate::util::{json_to_py, load_py_config, py_to_json};
use crate::window::WindowAttributesConfig;

mod config;
mod util;
mod webview;
mod window;
mod init_script;

fn get_wry_response(
    request: Request<Vec<u8>>,
    responder: RequestAsyncResponder,
    base_path: &PathBuf,
) {
    let path = request.uri().path();
    let relative_path = if path == "/" {
        "index.html"
    } else {
        &path[1..]
    };

    let file_path = base_path.join(relative_path);
    println!("Assets:: {}", request.uri());
    if !file_path.exists() {
        responder.respond(
            Response::builder()
                .header(CONTENT_TYPE, "text/plain")
                .status(404)
                .body(b"File not found".to_vec())
                .unwrap(),
        );
        return;
    }

    match fs::read(&file_path) {
        Ok(content) => {
            let mimetype = match file_path.extension().and_then(|ext| ext.to_str()) {
                Some("html") => "text/html",
                Some("js") => "text/javascript",
                Some("css") => "text/css",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("wasm") => "application/wasm",
                Some("json") => "application/json",
                _ => "application/octet-stream",
            };

            responder.respond(
                Response::builder()
                    .header(CONTENT_TYPE, mimetype)
                    .status(200)
                    .body(content)
                    .unwrap(),
            );
        }
        Err(_) => {
            responder.respond(
                Response::builder()
                    .header(CONTENT_TYPE, "text/plain")
                    .status(500)
                    .body(b"Failed to read the file".to_vec())
                    .unwrap(),
            );
        }
    }
}

fn create_new_window(
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
    let app = WindowBuilder::new()
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
    let webview = builder.with_initialization_script(get_init_script()).build(&app).unwrap();
    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        builder.build_gtk(vbox).unwrap()
    };
    (app, webview)
}

#[derive(Debug)]
enum UserEvent {
    Response(ResponseData),
    Emit(EmitData),
    Exit(),
    Close(WindowId),
    UpdateWindow(String, Value, bool),
}

#[derive(Serialize, Deserialize)]
struct IPCData {
    event_type: String,
    command: String,
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

#[pyclass(unsendable)]
struct WindowManager {
    webviews: Arc<Mutex<HashMap<String, WindowId>>>,
    command: Arc<Mutex<Py<PyFunction>>>,
    listener: Arc<Mutex<Py<PyFunction>>>,
    on_start: Arc<Mutex<Py<PyFunction>>>,
    on_stop: Arc<Mutex<Py<PyFunction>>>,
    config: Arc<Mutex<Config>>,
    proxy: Option<Arc<Mutex<EventLoopProxy<UserEvent>>>>,
    base_path: PathBuf,
    webview_windows: Arc<Mutex<HashMap<WindowId, (Window, WebView, String)>>>,
    event_loop: EventLoop<UserEvent>,

}

fn find_by_label<'a>(
    map: &'a HashMap<WindowId, (Window, WebView, String)>,
    label: &str,
) -> Option<(&'a WindowId, &'a (Window, WebView, String))> {
    map.iter().find(|(_, (_, _, lbl))| lbl == label)
}

#[pymethods]
impl WindowManager {
    #[new]
    #[pyo3(text_signature = "(command, listener,config_path, assets_dir)")]
    fn py_new(
        command: Py<PyFunction>,
        listener: Py<PyFunction>,
        on_start: Py<PyFunction>,
        on_stop: Py<PyFunction>,
        config: PyObject,
        assets_dir: String,
    ) -> PyResult<Self> {
        Ok(Self {
            webviews: Arc::new(Mutex::new(HashMap::new())),
            command: Arc::new(Mutex::new(command)),
            listener: Arc::new(Mutex::new(listener)),
            on_start: Arc::new(Mutex::new(on_start)),
            on_stop: Arc::new(Mutex::new(on_stop)),
            config: Arc::new(Mutex::new(load_py_config(config).unwrap())),
            proxy: None,
            base_path: PathBuf::from(assets_dir),
            webview_windows: Arc::new(Mutex::new(HashMap::new())),
            event_loop: EventLoopBuilder::<UserEvent>::with_user_event().build(),
        })
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
    fn close_window(&self, label: Option<String>) {
        if let Some(proxy) = self.proxy.clone() {
            if let Some(lbl) = label {
                if let Some(window_id) = self.webviews.clone().lock().unwrap().get(&lbl) {
                    proxy.lock().unwrap().send_event(UserEvent::Close(*window_id)).unwrap();
                }
            } else {
                proxy.lock().unwrap().send_event(UserEvent::Exit()).unwrap();
            }
        }
        // webview.evaluate_script().unwrap()
    }

    #[pyo3(text_signature = "(self)")]
    fn test_called_from_python(&self) {
        println!("Here :::");
    }

    #[pyo3(text_signature = "(self, label, updates)")]
    fn update_window(&self, py: Python, label: &str, updates: PyObject) -> PyResult<()> {
        println!("Here :::");
        // if let Some(proxy) = self.proxy.clone() {
        // let updates_json: Value = py_to_json(py, updates);
        // // proxy.lock().unwrap().send_event(UserEvent::UpdateWindow(label.to_string(), updates_json, false)).unwrap();
        // if let Some((_id, (window, webview, _label))) = find_by_label(&self.webview_windows.lock().unwrap(), label) {
        //     // Update other window properties
        //     if let Some(width) = updates_json.get("width").and_then(|v| v.as_i64()) {
        //         if let Some(height) = updates_json.get("height").and_then(|v| v.as_i64()) {
        //             window.set_inner_size(LogicalSize {
        //                 width: width as u32,
        //                 height: height as u32,
        //             });
        //         }
        //     }
        //     if let Some(resizable) = updates_json.get("resizable").and_then(|v| v.as_bool()) {
        //         window.set_resizable(resizable);
        //     }
        //     if let Some(minimizable) = updates_json.get("minimizable").and_then(|v| v.as_bool()) {
        //         window.set_minimizable(minimizable);
        //     }
        //     if let Some(maximizable) = updates_json.get("maximizable").and_then(|v| v.as_bool()) {
        //         window.set_maximizable(maximizable);
        //     }
        //     if let Some(closable) = updates_json.get("closable").and_then(|v| v.as_bool()) {
        //         window.set_closable(closable);
        //     }
        //     if let Some(fullscreen) = updates_json.get("fullscreen").and_then(|v| v.as_bool()) {
        //         window.set_fullscreen(if fullscreen {
        //             Some(tao::window::Fullscreen::Borderless(None))
        //         } else {
        //             None
        //         });
        //     }
        //     if let Some(visible) = updates_json.get("visible").and_then(|v| v.as_bool()) {
        //         window.set_visible(visible);
        //     }
        //     if let Some(always_on_top) = updates_json.get("always_on_top").and_then(|v| v.as_bool()) {
        //         window.set_always_on_top(always_on_top);
        //     }
        //     if let Some(title) = updates_json.get("title").and_then(|v| v.as_str()) {
        //         window.set_title(title);
        //     }
        //     if let Some(background_color) = updates_json.get("background_color").and_then(|v| {
        //         v.as_array().and_then(|arr| {
        //             if arr.len() == 4 {
        //                 Some((
        //                     arr[0].as_u64()? as u8,
        //                     arr[1].as_u64()? as u8,
        //                     arr[2].as_u64()? as u8,
        //                     arr[3].as_u64()? as u8,
        //                 ))
        //             } else {
        //                 None
        //             }
        //         })
        //     }) {
        //         window.set_background_color(Some(background_color));
        //     }
        // }
        Ok(())
        // };
    }

    #[pyo3(text_signature = "(self, label, updates)")]
    fn update_webview(&self, py: Python, label: &str, updates: PyObject) -> PyResult<()> {
        // if let Some(proxy) = self.proxy.clone() {
        let updates_json: Value = py_to_json(py, updates);
        // proxy.lock().unwrap().send_event(UserEvent::UpdateWindow(label.to_string(), updates_json, true)).unwrap();
        if let Some((_id, (window, webview, _label))) = find_by_label(&self.webview_windows.lock().unwrap(), label) {
            if let Some(url) = updates_json.get("url").and_then(|v| v.as_str()) {
                webview.load_url(url).unwrap();
            }
            if let Some(focus) = updates_json.get("focus").and_then(|v| v.as_bool()) {
                if focus {
                    webview.focus().expect("Error whe focusing webview")
                }
            }
            if let Some(script) = updates_json.get("script").and_then(|v| v.as_str()) {
                webview.evaluate_script(script).unwrap();
            }
            if let Some(visible) = updates_json.get("visible").and_then(|v| v.as_bool()) {
                webview.set_visible(visible).unwrap();
            }
            if let Some(html) = updates_json.get("html").and_then(|v| v.as_str()) {
                webview.load_html(html).unwrap();
            }
            if let Some(devtools) = updates_json.get("devtools").and_then(|v| v.as_bool()) {
                if devtools {
                    webview.close_devtools();
                } else {
                    webview.open_devtools();
                }
            }
            if let Some(clear) = updates_json.get("clear").and_then(|v| v.as_bool()) {
                if (clear) {
                    webview.clear_all_browsing_data().expect("Error when erased browser data");
                }
            }
        }
        Ok(())
        // };
    }


    #[pyo3(text_signature = "(self)")]
    fn run(&mut self) {
        let base_bath = self.base_path.clone();

        let config = self.config.lock().unwrap();
        let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();
        let command = self.command.clone();
        let listener = self.listener.clone();
        self.proxy = Some(Arc::new(Mutex::new(proxy.clone())));
        let on_start = self.on_start.clone();
        let on_stop = self.on_stop.clone();
        let webview_windows = Arc::clone(&self.webview_windows);
        let mut webview_cloned = self.webviews.clone();

        let protocol_handler: Arc<Mutex<Box<dyn Fn(WebViewId, Request<Vec<u8>>, RequestAsyncResponder) + Send + Sync>>> = Arc::new(Mutex::new(Box::new(move |id, request, responder| {
            get_wry_response(request, responder, &base_bath)
        })));

        let handler: Arc<Mutex<Box<dyn Fn(Request<String>)>>> = Arc::new(Mutex::new(Box::new(move |req: Request<String>| {
            let data: IPCData = serde_json::from_str(req.body()).unwrap();
            let listeners = listener.lock().unwrap();
            let commands = command.lock().unwrap();
            match data.event_type.as_str() {
                "event" => {
                    Python::with_gil(|py| {
                        let new_args = json!({"event": data.command, "args": data.args});
                        let args: PyObject = json_to_py(py, &new_args);
                        let py_args = PyTuple::new(py, &[args]).unwrap();
                        listeners.call1(py, py_args).unwrap();
                    });
                }
                "request" => {
                    Python::with_gil(|py| {
                        let new_args = json!({"command": data.command, "args": data.args});
                        let args: PyObject = json_to_py(py, &new_args);
                        let py_args = PyTuple::new(py, &[args]).unwrap();
                        let value = commands.call1(py, py_args).unwrap();
                        // proxy.clone().send_event(UserEvent::Response(ResponseData {
                        //     request_id: data.request_id,
                        //     data: Box::new(py_to_json(py, value)),
                        // })).unwrap();
                    });
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
                web_view,
                win.clone(),
                &event_loop,
                config.clone().icon.get_for_current_os(),
            );
            let window_id = new_window.0.id();
            let label = win.label.clone().unwrap_or_else(|| {
                format!(
                    "Window {}",
                    webview_cloned.lock().unwrap().len() + 1
                )
            });
            self.webview_windows.lock().unwrap().insert(window_id.clone(), (new_window.0, new_window.1, label.clone()));
            webview_cloned.lock().unwrap().insert(label, window_id.clone());
        }

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(StartCause::Init) => {
                    Python::with_gil(|py| {
                        let args: PyObject = json_to_py(py, &json!({"label": "main"}));
                        let py_args = PyTuple::new(py, &[args]).unwrap();
                        on_start.lock().unwrap().call1(py, py_args).unwrap()
                    });
                }
                Event::UserEvent(UserEvent::Response(data)) => {
                    for (_, webview) in webview_windows.lock().unwrap().iter().clone() {
                        let js_code = format!(
                            r#"
                                window.dispatchEvent(
                                    new CustomEvent('{}', {{
                                        detail: {{data: {} }}
                                    }})
                                );
                            "#,
                            data.request_id.as_str(), data.data.to_string()
                        );
                        webview.1.evaluate_script(js_code.as_str()).unwrap();
                    }
                }
                Event::UserEvent(UserEvent::Emit(data)) => {
                    for (_, webview) in webview_windows.lock().unwrap().iter().clone() {
                        let js_code = format!(
                            r#"
                                window.dispatchEvent(
                                    new CustomEvent('{}', {{
                                        detail: {{ data: {} }}
                                    }})
                                );
                            "#,
                            data.event.as_str(), data.data.to_string()
                        );
                        webview.1.evaluate_script(js_code.as_str()).unwrap();
                    }
                }
                Event::UserEvent(
                    UserEvent::Close(window_id)
                ) => {
                    let mut wm = webview_windows.lock().unwrap();
                    if let Some(w) = wm.remove(&window_id) {
                        let label = w.2.clone();
                        Python::with_gil(|py| {
                            let args: PyObject = json_to_py(py, &json!({"label": label}));
                            let py_args = PyTuple::new(py, &[args]).unwrap();
                            on_stop.lock().unwrap().call1(py, py_args).unwrap()
                        });
                    }
                    if wm.len() == 0 {
                        println!("Pywui exit ....");
                        *control_flow = ControlFlow::Exit
                    }
                }
                Event::UserEvent(
                    UserEvent::Exit()
                ) => {
                    println!("Pywui exit ....");
                    Python::with_gil(|py| {
                        let args: PyObject = json_to_py(py, &json!({"label": "main"}));
                        let py_args = PyTuple::new(py, &[args]).unwrap();
                        on_stop.lock().unwrap().call1(py, py_args).unwrap()
                    });
                    *control_flow = ControlFlow::Exit
                }
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    let mut wwin = webview_windows.lock().unwrap();
                    if let Some(ww) = wwin.remove(&window_id) {
                        let label = ww.2.clone();
                        Python::with_gil(|py| {
                            let args: PyObject = json_to_py(py, &json!({"label": label}));
                            let py_args = PyTuple::new(py, &[args]).unwrap();
                            on_stop.lock().unwrap().call1(py, py_args).unwrap()
                        });
                        if wwin.len() == 0 {
                            *control_flow = ControlFlow::Exit
                        } else if (ww.2.eq("main")) {
                            println!("Main exit");
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
                _ => {}
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
