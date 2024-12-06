from typing import Callable, Union


class WindowManager:

    def __init__(
            self,
            command: Callable,
            listener: Callable,
            on_start: Callable,
            on_stop: Callable,
            config: dict,
            assets_dir: str,
    ): ...

    def emit(self, event: str, data: any): ...

    def close_window(self, label: Union[str, None] = None): ...

    def update_window(self, label: str, updates: dict): ...

    def update_webview(self, label: str, updates: dict): ...

    def run(self): ...
    def test_called_from_python(self): ...
