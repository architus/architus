class Test:
    @property
    def x(self):
        return "hi"
    @x.setter
    def hi(self, value):
        print(f"got {value} for x")

    def __setattr__(self, name, value):
        print(f"tried to set {name} to {value}")

    def __getattr__(self, name):
        pass




t = Test()

t.x = 'hello'
print(t.x)
