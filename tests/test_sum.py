from pywui_rs import PyWui

app = PyWui()


@app.command("test")
def test():
    print("Hello")


@app.command("test2")
def test2(name):
    value = f"Hello {name}"
    print(value)
    return value


@app.listener("listener")
def listener1(name):
    value = f"Listener {name}"
    print(value)


@app.on_start
def on_start():
    print("App started ...")
    print(app._manager.close_window("main"))


@app.on_stop
def on_stop():
    print("App stopped ....")


if __name__ == '__main__':
    app.run()
