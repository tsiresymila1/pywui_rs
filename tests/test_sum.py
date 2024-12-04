import os.path

from pywui_rs import PyWui

app = PyWui(assets_dir=os.path.join(os.path.dirname(__file__), "assets"))


@app.command("test")
def test():
    print("Hello")


@app.command("test2")
def test2(name):
    value = f"Hello {name}"
    print(value)
    return value


app.run()
