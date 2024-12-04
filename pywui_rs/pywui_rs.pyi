from typing import Callable


class WindowManager:

    def __init__(self, command: Callable, listener: Callable, config_path: str, assets_dir: str): ...

    def run(self): ...
