from .pywui_rs import WindowManager

from .webview import Webview


class Window:
    def __init__(self, label: str, manager: WindowManager):
        self._manager = manager
        self._webview = Webview(label, manager)
        self._label = label

    def get_webview(self) -> Webview:
        return self._webview

    def resize(self, width: int, height: int):
        self._manager.update_window(
            self._label, {
                "width": width,
                "height": height
            })

    def fullscreen(self, full: bool):
        self._manager.update_window(
            self._label, {
                "fullscreen": full,
            })

    def show(self):
        self._manager.update_window(
            self._label, {
                "visible": True,
            })

    def hide(self):
        self._manager.update_window(
            self._label, {
                "visible": False,
            })

    def always_on_top(self, value: bool):
        self._manager.update_window(
            self._label, {
                "always_on_top": value,
            })

    def closable(self, value: bool):
        self._manager.update_window(
            self._label, {
                "closable": value,
            })

    def minimizable(self, value: bool):
        self._manager.update_window(
            self._label, {
                "minimizable": value,
            })

    def maximizable(self, value: bool):
        self._manager.update_window(
            self._label, {
                "maximizable": value,
            })

    def close(self):
        self._manager.close_window(self._label)

    def set_title(self, title: str):
        self._manager.update_window(
            self._label, {
                "title": title,
            })

    def background_color(self, r: int, g: int, b: int, a: int):
        self._manager.update_window(
            self._label, {
                "background_color": [r, g, b, a],
            })
