## Ion - A "rusted" electron

Current status: Horribly broken
Goal: Gui toolkit allowing apps to be built in native rust, with html/css/js display logic

TODO:
- Split into lib and make example hello world click counter application with rust click listener
- Component + databinding example: todolist
- Save button for todolist

- DEMO: Build simple text editor:
    - Matching brackets highlighting
    - Native menu, open file dialog
    - Find component

- OpenGL support: Allow overlaying opengl content, maybe hook into present?
- DEMO: Click button to control color of opengl window

To build:
Download a my fork of servo to ../servo
On Mac, build as normal.
On Fedora 27, I needed to use these env flags to get glutin to work. Hopefully this weirdness will be sorted out soon.
Also, for some reason, on Fedora a release build crashes with SIGILL
```
RUST_BACKTRACE=full
WAYLAND_DISPLAY=wayland-1
```

To upgrade servo:
change cargo.toml, then `cp -r ../servo/resources . && cp ../servo/Cargo.lock . && cp ../servo/rust-toolchain .`