from pywui_rs import PyWui

app = PyWui()


@app.command("test")
def test():
    print("Hello")
    app._manager.test_called_from_python()


@app.command("test2")
def test2(name):
    value = f"Hello {name}"
    print(value)
    return value


@app.on_start
def on_start():
    print("App started ...")


@app.on_stop
def on_stop():
    print("App stopped ....")


app.run()
