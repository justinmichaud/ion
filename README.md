## Ion - A "rusted" electron

Current status: Horribly broken
Goal: Gui toolkit allowing apps to be built in native rust, with html/css/js display logic

TODO:
- Spike - From rust:
    - Construct JS object in rust and use as param to call js method
    - Add JS Application object implemented in rust

What is proven possible:
- Changing attributes of element, inserting/deleting dom elements, getting value, running JS
- Registering JS onclick handlers

What is needed:
- Handle window on* events in rust
- Add rust callbacks for events (or use message passing)
- Construct JS object and use to call window method
- Send event to rust code: Application.cast("MyController.submit")
- Rust macro to generate html page from components?

- Design:
    - Allow app developers to register actor for every page, which can send/receive events from js

- Demo:
    - Simple notepad app

- OpenGL support: Allow overlaying opengl content, maybe hook into present?

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