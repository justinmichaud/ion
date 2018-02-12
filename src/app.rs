use std::ops::Deref;
use std::rc::Rc;

use servo::script::dom::document::Document;
use servo::script::dom::bindings::str::DOMString;
use servo::script::dom::eventtarget::EventTarget;
use servo::script::dom::eventtarget::RustEventHandler;
use servo::script::dom::bindings::codegen::Bindings::DocumentBinding::DocumentMethods;
use servo::script::dom::bindings::codegen::Bindings::ElementBinding::ElementMethods;
use servo::script::dom::bindings::inheritance::Castable;
use servo::script::dom::node::Node;
use servo::script::dom::bindings::codegen::Bindings::NodeBinding::NodeMethods;
use servo::script::dom::bindings::root::DomRoot;
use servo::script::dom::bindings::codegen::Bindings::DocumentBinding::ElementCreationOptions;
use servo::script::dom::element::Element;

use servo::script::script_thread::ION_APPLICATION_FRAME_CALLBACK;

use std::cell::RefCell;

thread_local!(static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new()));

struct AppState {
    count: i32,
    observers: Vec<fn(&Document, &mut AppState)->()>,
    has_changed: bool,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            count: 0,
            observers: vec![],
            has_changed: true,
        }
    }

    pub fn on_change(&mut self, observer: fn(&Document, &mut AppState)->()) {
        self.observers.push(observer)
    }

    pub fn get_count(&self) -> i32 { self.count }

    pub fn set_count(&mut self, count: i32) {
        self.count = count;
        self.has_changed = true;
    }

    pub fn tick(&mut self, doc: &Document) {
        if !self.has_changed { return; }
        let current_observers = self.observers.clone();
        for f in current_observers {
            f(doc, self);
        }
        self.has_changed = false;
    }
}

fn frame_callback(doc: &Document) {
    APP_STATE.with(|root| root.borrow_mut().tick(doc))
}

fn make_elem(doc: &Document, tag: &str, id: &str) -> DomRoot<Element> {
    let e: DomRoot<Element> = doc.CreateElement(DOMString::from_string(tag.to_string()),
                                                     unsafe { &ElementCreationOptions::empty(doc.window().get_cx()) }).unwrap();
    e.deref().SetId(DOMString::from_string(id.to_string()));
    e
}

pub fn app_main(doc: &Document) {
    ION_APPLICATION_FRAME_CALLBACK.with(|root| root.set(Some(frame_callback)));

    let window = doc.window();
    window.deref().upcast::<EventTarget>().add_event_handler_rust("load", RustEventHandler {
        handler: Rc::new( |doc: &Document, _| {
            let body_collection = doc.GetElementsByTagName(DOMString::from_string("body".to_string()));
            let body_ptr = body_collection.elements_iter().last().unwrap();

            body_ptr.deref().upcast::<Node>().AppendChild(&DomRoot::upcast(make_elem(&doc, "p", "text"))).unwrap();
            body_ptr.deref().upcast::<Node>().AppendChild(&DomRoot::upcast(make_elem(&doc, "button", "button"))).unwrap();

            APP_STATE.with(|root| {
                root.borrow_mut().on_change(|doc, state| {
                    let elem_ptr = doc.GetElementById(DOMString::from_string("text".to_string())).unwrap();
                    elem_ptr.deref().SetInnerHTML(DOMString::from_string(format!("The current count is {}!",
                        state.get_count()).to_string())).unwrap();
                });
                root.borrow_mut().tick(&doc);
            });

            let button_ptr = doc.GetElementById(DOMString::from_string("button".to_string())).unwrap();
            button_ptr.deref().SetInnerHTML(DOMString::from_string("Click me!".to_string())).unwrap();
            let node: &EventTarget = button_ptr.deref().upcast::<EventTarget>();
            node.add_event_handler_rust("click", RustEventHandler {
                handler: Rc::new(|_, _| {
                    APP_STATE.with(|root| {
                        let count = root.borrow().get_count();
                        root.borrow_mut().set_count(count + 1);
                    });
                })
            });
        })
    });
}