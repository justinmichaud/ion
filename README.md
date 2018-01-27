To build:
On Fedora 27, I needed to use these env flags to get glutin to work. Hopefully this weirdness will be sorted out soon
```
RUST_BACKTRACE=full
WAYLAND_DISPLAY=wayland-1
LD_LIBRARY_PATH=~/servo-libs # Possibly not needed, taken from a ubuntu machine, see https://github.com/PistonDevelopers/piston/issues/1202
```

To upgrade servo:
change cargo.toml, then `cp -r ../servo/resources . && cp ../servo/Cargo.lock . && cp ../servo/rust-toolchain .`