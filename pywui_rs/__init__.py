from functools import wraps
from typing import Callable

from .pywui_rs import WindowManager

__all__ = [
    "WindowManager"
]


class PyWui:
    def __init__(self):
        self.manager = WindowManager()

    def command(self, name: str):
        def decorator(callback: Callable):
            self.manager.add_command(name, callback)

            @wraps(callback)
            async def wrapper(*args, **kwargs):
                return await callback(*args, **kwargs)

            return wrapper

        return decorator

    def listener(self, event: str):
        def decorator(callback: Callable):
            self.manager.add_listener(event, callback)

            @wraps(callback)
            async def wrapper(*args, **kwargs):
                return await callback(*args, **kwargs)

            return wrapper

        return decorator

    def run(self):
        self.manager.run()
