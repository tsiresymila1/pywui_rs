from typing import Callable


class WindowManager:

    def __init__(self, config_path: str): ...
    def add_command(self, name: str, callback: Callable): ...

    def add_listener(self, name: str, callback: Callable): ...

    def run(self): ...
