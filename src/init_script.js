window.__pywui__ = {
    invoke(command, args = [], timeout = 30000) {
        return new Promise((resolve, reject) => {
            const request_id = `req_${ Date.now() }_${ Math.random() }`;
            const message = { type: "command", command, args, request_id };
            window.ipc.postMessage(JSON.stringify(message));
            const timer = setTimeout(() => reject(new Error("Timeout")), timeout);
            window.addEventListener(request_id, (ev) => {
                if (ev.detail) {
                    const error = ev.detail["error"];
                    const result = ev.detail["result"];
                    if (error) reject(new Error(error));
                    else resolve(result);
                    clearTimeout(timer);
                    window.removeEventListener(request_id, () => {
                    })
                }
            })
        });
    },
};
