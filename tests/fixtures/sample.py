import os
import sys

class Greeter:
    def greet(self, name: str) -> str:
        return f"Hello, {name}!"

def main():
    g = Greeter()
    print(g.greet("world"))

if __name__ == "__main__":
    main()
