## Ion - An Oxidized Electron
Proof of concept for building native html/css/rust apps using servo. Ideally, this would become electron, but with rust/servo instead of javascript/webkit

## Structure
You give servo an app_main function that is called once servo's script thread has started.
You can use it to manipulate the dom, add rust event handlers, or send messages to other processes for heavy lifting.

## TODO:
- *Find a way to not mutilate servo's encapsulation
- Component + databinding example: todolist that read/saves from file

- DEMO: Build simple text editor:
    - Native menu, open file dialog
    - Find/replace, matching bracket highlighting

- OpenGL support: Allow overlaying opengl content, maybe hook into window.present? Canvas would be nice.

To build:
Download https://github.com/justinmichaud/servo to ../servo
On Mac, build as normal with cargo.
On Fedora 27, I needed to use these env flags to get glutin to work. This seems to be some mesa bug, which should be fixed soon.
Also, for some reason, on Fedora a release build crashes with SIGILL
```
RUST_BACKTRACE=full
WAYLAND_DISPLAY=wayland-1
```

To upgrade servo:
change cargo.toml, then `cp -r ../servo/resources . && cp ../servo/Cargo.lock . && cp ../servo/rust-toolchain .`