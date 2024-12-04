pub fn get_init_script() -> &'static str {
    r#"
        window.__pywui__ = {
            invoke(command, args = [], timeout = 5000) {
                return new Promise((resolve, reject) => {
                    const request_id = `req_${ Date.now() }_${ Math.random() }`;
                    const message = { event_type: "request", command, args, request_id };
                    const timer = setTimeout(() => reject(new Error("Timeout")), timeout);
                    window.addEventListener(request_id, (ev) => {
                        if (ev.detail) {
                            const error = ev.detail["error"];
                            const result = ev.detail["data"];
                             clearTimeout(timer);
                             window.removeEventListener(request_id, () => {})
                            if (error) reject(new Error(error));
                            else resolve(result);
                        }
                    })
                    window.ipc.postMessage(JSON.stringify(message));
                });
            },
        };
    "#
}
