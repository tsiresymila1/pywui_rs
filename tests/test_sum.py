from pywui_rs import PyWui

app = PyWui()


@app.command("test")
def test():
    print("Hello")


app.run()
