Ion - A "rusted" electron

Current status: Horribly broken
Goal: Gui toolkit allowing apps to be built in native rust, with html/css/js display logic

TODO:
- Spike - From rust:
    - Find and mutate dom element
    - Add element to dom
    - Register rust callback or receive message for window onload event
    - Add button to page with rust onclick callback
    - load webpage from local resources folder
    - call rust code from js (send custom events?)
    - overlay opengl content

- Design:
    - Allow app developers to register actor for every page, which can send/receive events from js

- Demo:
    - Simple notepad app

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