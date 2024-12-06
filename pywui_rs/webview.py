from .pywui_rs import WindowManager


class Webview:
    def __init__(self, label: str, manager: WindowManager, devtools: bool = False):
        self._manager = manager
        self._label = label
        self._manager.update_webview(self._label, {
            "devtools": devtools
        })

    def eval(self, script: str):
        self._manager.update_webview(self._label, {
            "script": script
        })

    def load_url(self, url: str):
        self._manager.update_webview(self._label, {
            "url": url
        })

    def load_html(self, html: str):
        self._manager.update_webview(self._label, {
            "html": html
        })

    def clear_data(self):
        self._manager.update_webview(self._label, {
            "clear": True
        })

    def show(self):
        self._manager.update_webview(
            self._label, {
                "visible": True,
            })

    def hide(self):
        self._manager.update_webview(
            self._label, {
                "visible": False,
            })

    def devtools(self, enable: bool):
        self._manager.update_webview(
            self._label, {
                "devtools": enable,
            })
