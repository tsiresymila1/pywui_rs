import os
from functools import wraps
from typing import Callable, Union

from .pywui_rs import WindowManager

__all__ = [
    "WindowManager"
]


class PyWui:
    def __init__(self, config_path: Union[str, None] = None):
        full_path = os.path.join(config_path or os.getcwd(), 'pywui.conf.json')
        if not os.path.exists(full_path):
            raise Exception(f"{full_path} not exist")
        self._manager = WindowManager(config_path=full_path)

    def command(self, name: str):
        def decorator(callback: Callable):
            self._manager.add_command(name, callback)

            @wraps(callback)
            async def wrapper(*args, **kwargs):
                return await callback(*args, **kwargs)

            return wrapper

        return decorator

    def listener(self, event: str):
        def decorator(callback: Callable):
            self._manager.add_listener(event, callback)

            @wraps(callback)
            async def wrapper(*args, **kwargs):
                return await callback(*args, **kwargs)

            return wrapper

        return decorator

    def run(self):
        self._manager.run()
