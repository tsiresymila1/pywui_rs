import json
import os
import uuid
from functools import wraps
from typing import Callable, Union, Any

from .pywui_rs import WindowManager
from .window import Window

__all__ = [
    "WindowManager"
]


class PyWui:
    _commands: dict[str, Callable] = {}
    _listeners: dict[str, list[Callable]] = {}
    _on_start_listener: list[Callable] = []
    _on_stop_listener: list[Callable] = []
    _windows: dict[str, Window] = {}

    def __init__(self, config_path: Union[str, None] = None):
        full_path = os.path.join(config_path or os.getcwd(), 'pywui.conf.json')
        if not os.path.exists(full_path):
            raise Exception(f"{full_path} not exist")

        def handle_request(info: dict):
            return self._handler_request(info)

        def handle_event(info: dict):
            return self._handler_event(info)

        def handle_start(info: dict = None):
            return self._on_start(info)

        def handle_stop(info: dict = None):
            return self._on_stop(info)

        config_dict = self._parse_config(self._load_config(full_path))

        assets_dir = config_dict.get("build", {}).get("buildPath")
        self._manager = WindowManager(
            command=handle_request,
            listener=handle_event,
            on_start=handle_start,
            on_stop=handle_stop,
            config=config_dict,
            assets_dir=assets_dir
        )
        print(self._manager)
        self._create_windows(config_dict)

    @classmethod
    def _load_config(cls, path: str) -> dict:
        with open(path, 'r') as f:
            text = f.read()
            return json.loads(text)

    @classmethod
    def _parse_config(cls, config: dict) -> dict:
        pywui: Union[dict, None] = config.get("pywui", {})
        if pywui:
            windows: list[dict] = pywui.get("windows", [])
            new_windows: list[dict] = []
            main_exist: bool = any([w['label'] == "main" for w in windows])

            for key, win in enumerate(windows):
                label = win.get("label", None)
                wind_copy = win.copy()
                if not label:
                    main_exist_new: bool = any([w['label'] == "main" for w in new_windows])
                    if not main_exist and not main_exist_new:
                        wind_copy['label'] = "main"
                    else:
                        wind_copy['label'] = str(uuid.uuid4())
                new_windows.append(wind_copy)
            pywui['windows'] = new_windows
        cfg_copy = config.copy()
        cfg_copy['pywui'] = pywui
        return cfg_copy

    def _create_windows(self, config: dict):
        pywui: Union[dict, None] = config.get("pywui")
        if pywui:
            windows: list[dict] = pywui.get("windows", [])
            for win in windows:
                label: str = win.get("label")
                self._windows[label] = Window(label, self._manager)

    def get_window(self, label: str = "main") -> Union[Window, None]:
        return self._windows.get(label)

    def _add_command(self, name: str, callback: Callable):
        self._commands[name] = callback

    def _add_listener(self, name: str, callback: Callable):
        elements: list = self._listeners.get(name, [])
        elements.append(callback)
        self._listeners[name] = elements

    @classmethod
    def _create_response(cls, data: Any, error: Any = None) -> dict:
        return {
            "error": error,
            "data": data
        }

    def _on_start(self, info: dict = None):
        for callback in self._on_start_listener:
            try:
                callback()
            except Exception as e:
                print("Error", e)

    def _on_stop(self, info: dict = None):
        for callback in self._on_stop_listener:
            try:
                callback()
            except Exception as e:
                print("Error", e)

    def _handler_request(self, info: dict):
        args = info['args']
        command = info["command"]
        try:
            if command in self._commands:
                command_handler = self._commands.get(command)
                result = command_handler(*args)
                return self._create_response(result)
            else:
                return self._create_response(None, "Command found")
        except Exception as e:
            print("Error : ", str(e))
            return self._create_response(None, str(e))

    def _handler_event(self, info: dict):
        args = info['args']
        event = info["event"]
        if event in self._listeners:
            listeners = self._listeners.get(event)
            for listener in listeners:
                try:
                    listener(*args)
                except Exception as e:
                    print("Error", e)

    def command(self, name: str):
        def decorator(callback: Callable):
            self._add_command(name, callback)

            @wraps(callback)
            async def wrapper(*args, **kwargs):
                return await callback(*args, **kwargs)

            return wrapper

        return decorator

    def listener(self, event: str):
        def decorator(callback: Callable):
            self._add_listener(event, callback)

            @wraps(callback)
            async def wrapper(*args, **kwargs):
                return await callback(*args, **kwargs)

            return wrapper

        return decorator

    def on_start(self, callback: Callable):
        self._on_start_listener.append(callback)
        return callback

    def on_stop(self, callback: Callable):
        self._on_stop_listener.append(callback)
        return callback

    def run(self):
        self._manager.run()
