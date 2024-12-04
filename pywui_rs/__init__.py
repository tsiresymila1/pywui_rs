import os
from functools import wraps
from typing import Callable, Union

from .pywui_rs import WindowManager

__all__ = [
    "WindowManager"
]


class PyWui:
    _commands: dict[str, Callable] = {}
    _listeners: dict[str, list[Callable]] = {}

    def __init__(self, assets_dir: str, config_path: Union[str, None] = None):
        full_path = os.path.join(config_path or os.getcwd(), 'pywui.conf.json')
        if not os.path.exists(full_path):
            raise Exception(f"{full_path} not exist")

        def handle_request(info: dict):
            return self._handler_request(info)

        def handle_event(info: dict):
            return self._handler_request(info)

        self._manager = WindowManager(
            command=handle_request,
            listener=handle_event,
            config_path=full_path,
            assets_dir=assets_dir
        )

    def _add_command(self, name: str, callback: Callable):
        self._commands[name] = callback

    def _add_listener(self, name: str, callback: Callable):
        elements: list = self._listeners.get(name, [])
        elements.append(callback)
        self._listeners[name] = elements

    def _handler_request(self, info: dict):
        print("Handle request ::", info)
        args = info['args']
        command = info["command"]
        if command in self._commands:
            command_handler = self._commands.get(command)
            result = command_handler(*args)
            print("Command result :::", result)
            return result

    def _handler_event(self, info: dict):
        print("Handle event ::", info)
        args = info['args']
        event = info["event"]
        if event in self._listeners:
            listeners = self._listeners.get(event)
            for listener in listeners:
                listener(*args)

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

    def run(self):
        self._manager.run()
