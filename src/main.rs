#![feature(plugin)]
#![plugin(interpolate_idents)]

#[macro_use]
mod observable;
#[macro_use]
mod html;
mod app;

#[macro_use] extern crate maplit;

/*
    This is taken mostly from https://github.com/paulrouget/servo-embedding-example
*/

extern crate glutin;
extern crate servo;

use servo::gl;
use glutin::GlContext;
use servo::BrowserId;
use servo::compositing::compositor_thread::EventLoopWaker;
use servo::compositing::windowing::{WindowEvent, WindowMethods, MouseWindowEvent};
use servo::euclid::{Point2D, TypedScale, Size2D, TypedPoint2D, TypedRect, TypedSize2D,
                    TypedVector2D};
use servo::ipc_channel::ipc;
use servo::msg::constellation_msg::{Key, KeyModifiers, TopLevelBrowsingContextId};
use servo::net_traits::net_error_list::NetError;
use servo::script_traits::{LoadData, TouchEventType, MouseButton};
use servo::servo_config::opts;
use servo::servo_config::resource_files::set_resources_path;
use servo::servo_geometry::DeviceIndependentPixel;
use servo::servo_url::ServoUrl;
use servo::style_traits::DevicePixel;
use servo::style_traits::cursor::CursorKind;
use servo::msg::constellation_msg::KeyState;
use std::env;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::Cell;

pub struct GlutinEventLoopWaker {
    proxy: Arc<glutin::EventsLoopProxy>,
}

impl EventLoopWaker for GlutinEventLoopWaker {
    // Use by servo to share the "event loop waker" across threads
    fn clone(&self) -> Box<EventLoopWaker + Send> {
        Box::new(GlutinEventLoopWaker { proxy: self.proxy.clone() })
    }
    // Called by servo when the main thread needs to wake up
    fn wake(&self) {
        self.proxy.wakeup().expect("wakeup eventloop failed");
    }
}

struct Window {
    glutin_window: glutin::GlWindow,
    waker: Box<EventLoopWaker>,
    gl: Rc<gl::Gl>,
}

fn main() {
    println!("Servo version: {}", servo::config::servo_version());

    let mut event_loop = glutin::EventsLoop::new();

    let builder = glutin::WindowBuilder::new().with_dimensions(800, 600);
    let gl_version = glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2));
    let context = glutin::ContextBuilder::new()
        .with_gl(gl_version)
        .with_vsync(true);
    let window = glutin::GlWindow::new(builder, context, &event_loop).unwrap();

    window.show();

    let gl = unsafe {
        window
            .context()
            .make_current()
            .expect("Couldn't make window current");
        gl::GlFns::load_with(|s| window.context().get_proc_address(s) as *const _)
    };

    let event_loop_waker =
        Box::new(GlutinEventLoopWaker { proxy: Arc::new(event_loop.create_proxy()) });

    let path = env::current_dir().unwrap().join("resources");
    let path = path.to_str().unwrap().to_string();
    set_resources_path(Some(path));
    opts::set_defaults(opts::default_opts());

    let window = Rc::new(Window {
                             glutin_window: window,
                             waker: event_loop_waker,
                             gl,
                         });

    let mut servo = servo::Servo::new(window.clone(), Some((app::app_setup, html::app_main)));

    let url = ServoUrl::parse(&format!("file://{}",  env::current_dir().unwrap()
        .join("app_resources/index.html").to_str().unwrap())).unwrap();
    let (sender, receiver) = ipc::channel().unwrap();
    servo.handle_events(vec![WindowEvent::NewBrowser(url, sender)]);
    let browser_id = receiver.recv().unwrap();
    servo.handle_events(vec![WindowEvent::SelectBrowser(browser_id)]);

    let mut pointer = (0.0, 0.0);

    let key_modifiers: Cell<KeyModifiers> = Cell::new(KeyModifiers::empty());
    let last_pressed_key: Cell<Option<Key>> = Cell::new(None);

    event_loop.run_forever(|event| {
        // Blocked until user event or until servo unblocks it
        match event {
            // This is the event triggered by GlutinEventLoopWaker
            glutin::Event::Awakened => {
                servo.handle_events(vec![]);
            }

            glutin::Event::WindowEvent {
                event: glutin::WindowEvent::Closed, ..
            } => {
                servo.handle_events(vec![WindowEvent::CloseBrowser(browser_id)]);
                return glutin::ControlFlow::Break
            },

            // Mousemove
            glutin::Event::WindowEvent {
                event: glutin::WindowEvent::CursorMoved { position: (x, y), .. }, ..
            } => {
                pointer = (x, y);
                let event = WindowEvent::MouseWindowMoveEventClass(TypedPoint2D::new(x as f32,
                                                                                     y as f32));
                servo.handle_events(vec![event]);
            }

            glutin::Event::WindowEvent {
                event: glutin::WindowEvent::MouseInput {
                    button: glutin::MouseButton::Left,
                    state: glutin::ElementState::Pressed, ..
                }, ..
            } => {
                let (x, y) = pointer;
                let event = WindowEvent::MouseWindowEventClass(MouseWindowEvent::Click(
                    MouseButton::Left, TypedPoint2D::new(x as f32, y as f32)
                ));
                servo.handle_events(vec![event]);
            }

            // Scrolling
            glutin::Event::WindowEvent {
                event: glutin::WindowEvent::MouseWheel { delta, phase, .. }, ..
            } => {
                let pointer = TypedPoint2D::new(pointer.0 as i32, pointer.1 as i32);
                let (dx, dy) = match delta {
                    glutin::MouseScrollDelta::LineDelta(dx, dy) => {
                        (dx, dy * 38.0 /*line height*/)
                    }
                    glutin::MouseScrollDelta::PixelDelta(dx, dy) => (dx, dy),
                };
                let scroll_location =
                    servo::webrender_api::ScrollLocation::Delta(TypedVector2D::new(dx, dy));
                let phase = match phase {
                    glutin::TouchPhase::Started => TouchEventType::Down,
                    glutin::TouchPhase::Moved => TouchEventType::Move,
                    glutin::TouchPhase::Ended => TouchEventType::Up,
                    glutin::TouchPhase::Cancelled => TouchEventType::Up,
                };
                let event = WindowEvent::Scroll(scroll_location, pointer, phase);
                servo.handle_events(vec![event]);
            }
            glutin::Event::WindowEvent {
                event: glutin::WindowEvent::Resized(width, height), ..
            } => {
                let event = WindowEvent::Resize;
                servo.handle_events(vec![event]);
                window.glutin_window.resize(width, height);
            }

            // from https://github.com/paulrouget/servoshell/blob/244631ccdc4525fe15f527fd0aa246fa298b168f/src/platform/glutin/mod.rs
            glutin::Event::WindowEvent {
                event: glutin::WindowEvent::KeyboardInput {
                    input: glutin::KeyboardInput {
                        state,
                        virtual_keycode: Some(virtual_keycode),
                        modifiers,
                        ..
                    },
                    ..
                },
                ..
            } => {
                let mut servo_mods = KeyModifiers::empty();
                if modifiers.shift {
                    servo_mods.insert(KeyModifiers::SHIFT);
                }
                if modifiers.ctrl {
                    servo_mods.insert(KeyModifiers::CONTROL);
                }
                if modifiers.alt {
                    servo_mods.insert(KeyModifiers::ALT);
                }
                if modifiers.logo {
                    servo_mods.insert(KeyModifiers::SUPER);
                }

                key_modifiers.set(servo_mods);

                if let Ok(key) = glutin_key_to_script_key(virtual_keycode) {
                    let state = match state {
                        glutin::ElementState::Pressed => KeyState::Pressed,
                        glutin::ElementState::Released => KeyState::Released,
                    };
                    if state == KeyState::Pressed {
                        if is_printable(virtual_keycode) {
                            last_pressed_key.set(Some(key));
                        }
                    }
                    servo.handle_events(vec![WindowEvent::KeyEvent(None, key, state, key_modifiers.get())]);
                }
            }

            glutin::Event::WindowEvent {
                event: glutin::WindowEvent::ReceivedCharacter(ch),
                ..
            } => {
                let mods = key_modifiers.get();

                let event = if let Some(last_pressed_key) = last_pressed_key.get() {
                    Some(WindowEvent::KeyEvent(Some(ch), last_pressed_key, KeyState::Pressed, mods))
                } else {
                    if !ch.is_control() {
                        match char_to_script_key(ch) {
                            Some(key) => {
                                Some(WindowEvent::KeyEvent(Some(ch), key, KeyState::Pressed, mods))
                            }
                            None => None,
                        }
                    } else {
                        None
                    }
                };
                last_pressed_key.set(None);

                if let Some(e) = event {
                    servo.handle_events(vec![e]);
                }
            }

            _ => {}
        }
        glutin::ControlFlow::Continue
    });
}

impl WindowMethods for Window {
    fn prepare_for_composite(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn present(&self) {
        self.glutin_window.swap_buffers().unwrap();
    }

    fn supports_clipboard(&self) -> bool {
        false
    }

    fn create_event_loop_waker(&self) -> Box<EventLoopWaker> {
        self.waker.clone()
    }

    fn gl(&self) -> Rc<gl::Gl> {
        self.gl.clone()
    }

    fn hidpi_factor(&self) -> TypedScale<f32, DeviceIndependentPixel, DevicePixel> {
        TypedScale::new(self.glutin_window.hidpi_factor())
    }

    fn framebuffer_size(&self) -> TypedSize2D<u32, DevicePixel> {
        let (width, height) = self.glutin_window.get_inner_size().unwrap();
        let scale_factor = self.glutin_window.hidpi_factor() as u32;
        TypedSize2D::new(scale_factor * width, scale_factor * height)
    }

    fn window_rect(&self) -> TypedRect<u32, DevicePixel> {
        TypedRect::new(TypedPoint2D::new(0, 0), self.framebuffer_size())
    }

    fn size(&self) -> TypedSize2D<f32, DeviceIndependentPixel> {
        let (width, height) = self.glutin_window.get_inner_size().unwrap();
        TypedSize2D::new(width as f32, height as f32)
    }

    fn client_window(&self, _id: BrowserId) -> (Size2D<u32>, Point2D<i32>) {
        let (width, height) = self.glutin_window.get_inner_size().unwrap();
        let (x, y) = self.glutin_window.get_position().unwrap();
        (Size2D::new(width, height), Point2D::new(x as i32, y as i32))
    }

    fn set_inner_size(&self, _id: BrowserId, _size: Size2D<u32>) {}

    fn set_position(&self, _id: BrowserId, _point: Point2D<i32>) {}

    fn set_fullscreen_state(&self, _id: BrowserId, _state: bool) {}

    fn set_page_title(&self, _id: BrowserId, title: Option<String>) {
        self.glutin_window
            .set_title(match title {
                           Some(ref title) => title,
                           None => "",
                       });
    }

    fn status(&self, _id: BrowserId, _status: Option<String>) {}

    fn allow_navigation(&self, _id: BrowserId, _url: ServoUrl, chan: ipc::IpcSender<bool>) {
        chan.send(true).ok();
    }

    fn load_start(&self, _id: BrowserId) {}

    fn load_end(&self, _id: BrowserId) {}

    fn load_error(&self, _id: BrowserId, _: NetError, _url: String) {}

    fn head_parsed(&self, _id: BrowserId) {}

    fn history_changed(&self, _id: BrowserId, _entries: Vec<LoadData>, _current: usize) {}

    fn set_favicon(&self, _id: BrowserId, _url: ServoUrl) {}

    fn handle_key(&self,
                  _id: Option<BrowserId>,
                  _ch: Option<char>,
                  _key: Key,
                  _mods: KeyModifiers) {
    }
    fn screen_size(&self, _ctx: TopLevelBrowsingContextId) -> Size2D<u32> {
        unimplemented!()
    }

    fn screen_avail_size(&self, _ctx: TopLevelBrowsingContextId) -> Size2D<u32> {
        unimplemented!()
    }

    fn set_cursor(&self, cursor: CursorKind) {
        let cursor = match cursor {
            CursorKind::Pointer => glutin::MouseCursor::Hand,
            _ => glutin::MouseCursor::Default,
        };
        self.glutin_window.set_cursor(cursor);
    }
}

// From https://github.com/paulrouget/servoshell/blob/244631ccdc4525fe15f527fd0aa246fa298b168f/src/platform/glutin/utils.rs

pub fn is_printable(key_code: glutin::VirtualKeyCode) -> bool {
    use glutin::VirtualKeyCode::*;
    match key_code {
        Escape | F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12 | F13 | F14 |
        F15 | Snapshot | Scroll | Pause | Insert | Home | Delete | End | PageDown | PageUp |
        Left | Up | Right | Down | Back | LAlt | LControl | LMenu | LShift | LWin | Mail |
        MediaSelect | MediaStop | Mute | MyComputer | NavigateForward | NavigateBackward |
        NextTrack | NoConvert | PlayPause | Power | PrevTrack | RAlt | RControl | RMenu |
        RShift | RWin | Sleep | Stop | VolumeDown | VolumeUp | Wake | WebBack | WebFavorites |
        WebForward | WebHome | WebRefresh | WebSearch | WebStop => false,
        _ => true,
    }
}

pub fn glutin_key_to_script_key(key: glutin::VirtualKeyCode) -> Result<Key, ()> {
    match key {
        glutin::VirtualKeyCode::A => Ok(Key::A),
        glutin::VirtualKeyCode::B => Ok(Key::B),
        glutin::VirtualKeyCode::C => Ok(Key::C),
        glutin::VirtualKeyCode::D => Ok(Key::D),
        glutin::VirtualKeyCode::E => Ok(Key::E),
        glutin::VirtualKeyCode::F => Ok(Key::F),
        glutin::VirtualKeyCode::G => Ok(Key::G),
        glutin::VirtualKeyCode::H => Ok(Key::H),
        glutin::VirtualKeyCode::I => Ok(Key::I),
        glutin::VirtualKeyCode::J => Ok(Key::J),
        glutin::VirtualKeyCode::K => Ok(Key::K),
        glutin::VirtualKeyCode::L => Ok(Key::L),
        glutin::VirtualKeyCode::M => Ok(Key::M),
        glutin::VirtualKeyCode::N => Ok(Key::N),
        glutin::VirtualKeyCode::O => Ok(Key::O),
        glutin::VirtualKeyCode::P => Ok(Key::P),
        glutin::VirtualKeyCode::Q => Ok(Key::Q),
        glutin::VirtualKeyCode::R => Ok(Key::R),
        glutin::VirtualKeyCode::S => Ok(Key::S),
        glutin::VirtualKeyCode::T => Ok(Key::T),
        glutin::VirtualKeyCode::U => Ok(Key::U),
        glutin::VirtualKeyCode::V => Ok(Key::V),
        glutin::VirtualKeyCode::W => Ok(Key::W),
        glutin::VirtualKeyCode::X => Ok(Key::X),
        glutin::VirtualKeyCode::Y => Ok(Key::Y),
        glutin::VirtualKeyCode::Z => Ok(Key::Z),

        glutin::VirtualKeyCode::Numpad0 => Ok(Key::Kp0),
        glutin::VirtualKeyCode::Numpad1 => Ok(Key::Kp1),
        glutin::VirtualKeyCode::Numpad2 => Ok(Key::Kp2),
        glutin::VirtualKeyCode::Numpad3 => Ok(Key::Kp3),
        glutin::VirtualKeyCode::Numpad4 => Ok(Key::Kp4),
        glutin::VirtualKeyCode::Numpad5 => Ok(Key::Kp5),
        glutin::VirtualKeyCode::Numpad6 => Ok(Key::Kp6),
        glutin::VirtualKeyCode::Numpad7 => Ok(Key::Kp7),
        glutin::VirtualKeyCode::Numpad8 => Ok(Key::Kp8),
        glutin::VirtualKeyCode::Numpad9 => Ok(Key::Kp9),

        glutin::VirtualKeyCode::Key0 => Ok(Key::Num0),
        glutin::VirtualKeyCode::Key1 => Ok(Key::Num1),
        glutin::VirtualKeyCode::Key2 => Ok(Key::Num2),
        glutin::VirtualKeyCode::Key3 => Ok(Key::Num3),
        glutin::VirtualKeyCode::Key4 => Ok(Key::Num4),
        glutin::VirtualKeyCode::Key5 => Ok(Key::Num5),
        glutin::VirtualKeyCode::Key6 => Ok(Key::Num6),
        glutin::VirtualKeyCode::Key7 => Ok(Key::Num7),
        glutin::VirtualKeyCode::Key8 => Ok(Key::Num8),
        glutin::VirtualKeyCode::Key9 => Ok(Key::Num9),

        glutin::VirtualKeyCode::Return => Ok(Key::Enter),
        glutin::VirtualKeyCode::Space => Ok(Key::Space),
        glutin::VirtualKeyCode::Escape => Ok(Key::Escape),
        glutin::VirtualKeyCode::Equals => Ok(Key::Equal),
        glutin::VirtualKeyCode::Minus => Ok(Key::Minus),
        glutin::VirtualKeyCode::Back => Ok(Key::Backspace),
        glutin::VirtualKeyCode::PageDown => Ok(Key::PageDown),
        glutin::VirtualKeyCode::PageUp => Ok(Key::PageUp),

        glutin::VirtualKeyCode::Insert => Ok(Key::Insert),
        glutin::VirtualKeyCode::Home => Ok(Key::Home),
        glutin::VirtualKeyCode::Delete => Ok(Key::Delete),
        glutin::VirtualKeyCode::End => Ok(Key::End),

        glutin::VirtualKeyCode::Left => Ok(Key::Left),
        glutin::VirtualKeyCode::Up => Ok(Key::Up),
        glutin::VirtualKeyCode::Right => Ok(Key::Right),
        glutin::VirtualKeyCode::Down => Ok(Key::Down),

        glutin::VirtualKeyCode::LShift => Ok(Key::LeftShift),
        glutin::VirtualKeyCode::LControl => Ok(Key::LeftControl),
        glutin::VirtualKeyCode::LAlt => Ok(Key::LeftAlt),
        glutin::VirtualKeyCode::LWin => Ok(Key::LeftSuper),
        glutin::VirtualKeyCode::RShift => Ok(Key::RightShift),
        glutin::VirtualKeyCode::RControl => Ok(Key::RightControl),
        glutin::VirtualKeyCode::RAlt => Ok(Key::RightAlt),
        glutin::VirtualKeyCode::RWin => Ok(Key::RightSuper),

        glutin::VirtualKeyCode::Apostrophe => Ok(Key::Apostrophe),
        glutin::VirtualKeyCode::Backslash => Ok(Key::Backslash),
        glutin::VirtualKeyCode::Comma => Ok(Key::Comma),
        glutin::VirtualKeyCode::Grave => Ok(Key::GraveAccent),
        glutin::VirtualKeyCode::LBracket => Ok(Key::LeftBracket),
        glutin::VirtualKeyCode::Period => Ok(Key::Period),
        glutin::VirtualKeyCode::RBracket => Ok(Key::RightBracket),
        glutin::VirtualKeyCode::Semicolon => Ok(Key::Semicolon),
        glutin::VirtualKeyCode::Slash => Ok(Key::Slash),
        glutin::VirtualKeyCode::Tab => Ok(Key::Tab),
        glutin::VirtualKeyCode::Subtract => Ok(Key::Minus),

        glutin::VirtualKeyCode::F1 => Ok(Key::F1),
        glutin::VirtualKeyCode::F2 => Ok(Key::F2),
        glutin::VirtualKeyCode::F3 => Ok(Key::F3),
        glutin::VirtualKeyCode::F4 => Ok(Key::F4),
        glutin::VirtualKeyCode::F5 => Ok(Key::F5),
        glutin::VirtualKeyCode::F6 => Ok(Key::F6),
        glutin::VirtualKeyCode::F7 => Ok(Key::F7),
        glutin::VirtualKeyCode::F8 => Ok(Key::F8),
        glutin::VirtualKeyCode::F9 => Ok(Key::F9),
        glutin::VirtualKeyCode::F10 => Ok(Key::F10),
        glutin::VirtualKeyCode::F11 => Ok(Key::F11),
        glutin::VirtualKeyCode::F12 => Ok(Key::F12),

        glutin::VirtualKeyCode::NavigateBackward => Ok(Key::NavigateBackward),
        glutin::VirtualKeyCode::NavigateForward => Ok(Key::NavigateForward),
        _ => Err(()),
    }
}

pub fn char_to_script_key(c: char) -> Option<Key> {
    match c {
        ' ' => Some(Key::Space),
        '"' => Some(Key::Apostrophe),
        '\'' => Some(Key::Apostrophe),
        '<' => Some(Key::Comma),
        ',' => Some(Key::Comma),
        '_' => Some(Key::Minus),
        '-' => Some(Key::Minus),
        '>' => Some(Key::Period),
        '.' => Some(Key::Period),
        '?' => Some(Key::Slash),
        '/' => Some(Key::Slash),
        '~' => Some(Key::GraveAccent),
        '`' => Some(Key::GraveAccent),
        ')' => Some(Key::Num0),
        '0' => Some(Key::Num0),
        '!' => Some(Key::Num1),
        '1' => Some(Key::Num1),
        '@' => Some(Key::Num2),
        '2' => Some(Key::Num2),
        '#' => Some(Key::Num3),
        '3' => Some(Key::Num3),
        '$' => Some(Key::Num4),
        '4' => Some(Key::Num4),
        '%' => Some(Key::Num5),
        '5' => Some(Key::Num5),
        '^' => Some(Key::Num6),
        '6' => Some(Key::Num6),
        '&' => Some(Key::Num7),
        '7' => Some(Key::Num7),
        '*' => Some(Key::Num8),
        '8' => Some(Key::Num8),
        '(' => Some(Key::Num9),
        '9' => Some(Key::Num9),
        ':' => Some(Key::Semicolon),
        ';' => Some(Key::Semicolon),
        '+' => Some(Key::Equal),
        '=' => Some(Key::Equal),
        'A' => Some(Key::A),
        'a' => Some(Key::A),
        'B' => Some(Key::B),
        'b' => Some(Key::B),
        'C' => Some(Key::C),
        'c' => Some(Key::C),
        'D' => Some(Key::D),
        'd' => Some(Key::D),
        'E' => Some(Key::E),
        'e' => Some(Key::E),
        'F' => Some(Key::F),
        'f' => Some(Key::F),
        'G' => Some(Key::G),
        'g' => Some(Key::G),
        'H' => Some(Key::H),
        'h' => Some(Key::H),
        'I' => Some(Key::I),
        'i' => Some(Key::I),
        'J' => Some(Key::J),
        'j' => Some(Key::J),
        'K' => Some(Key::K),
        'k' => Some(Key::K),
        'L' => Some(Key::L),
        'l' => Some(Key::L),
        'M' => Some(Key::M),
        'm' => Some(Key::M),
        'N' => Some(Key::N),
        'n' => Some(Key::N),
        'O' => Some(Key::O),
        'o' => Some(Key::O),
        'P' => Some(Key::P),
        'p' => Some(Key::P),
        'Q' => Some(Key::Q),
        'q' => Some(Key::Q),
        'R' => Some(Key::R),
        'r' => Some(Key::R),
        'S' => Some(Key::S),
        's' => Some(Key::S),
        'T' => Some(Key::T),
        't' => Some(Key::T),
        'U' => Some(Key::U),
        'u' => Some(Key::U),
        'V' => Some(Key::V),
        'v' => Some(Key::V),
        'W' => Some(Key::W),
        'w' => Some(Key::W),
        'X' => Some(Key::X),
        'x' => Some(Key::X),
        'Y' => Some(Key::Y),
        'y' => Some(Key::Y),
        'Z' => Some(Key::Z),
        'z' => Some(Key::Z),
        '{' => Some(Key::LeftBracket),
        '[' => Some(Key::LeftBracket),
        '|' => Some(Key::Backslash),
        '\\' => Some(Key::Backslash),
        '}' => Some(Key::RightBracket),
        ']' => Some(Key::RightBracket),
        _ => None,
    }
}

