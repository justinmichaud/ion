use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;

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

observable! {&Document, &mut AppState, struct AppStateModel {
    count : i32 = 0,
}}

struct AppState {
    model: AppStateModel,
    text: HtmlElement,
    button: HtmlElement,
}

thread_local!(static APP_STATE: RefCell<Option<AppState>> = RefCell::new(None));

#[derive(Clone)]
struct HtmlElement {
    id: String,
    tag: String,
    children: Vec<HtmlElement>,
    on_update: fn(&mut AppState, &Document)->(),
}

impl HtmlElement {
    pub fn new(tag: String, children: Vec<HtmlElement>, on_update: fn(&mut AppState, &Document)->()) -> HtmlElement {
        HtmlElement {
            id: HtmlElement::gen_id(),
            tag, children, on_update,
        }
    }

    fn gen_id() -> String {
        thread_local!(static ID_COUNTER: RefCell<u32> = RefCell::new(0));
        ID_COUNTER.with(|root| {
            let val = *root.borrow() + 1;
            *root.borrow_mut() += val;
            val.to_string()
        })
    }

    //TODO append_to_parent
    fn append_to_body(doc: &Document, elem: DomRoot<Element>) {
        let body_collection = doc.GetElementsByTagName(DOMString::from_string("body".to_string()));
        let body_ptr = body_collection.elements_iter().last().unwrap();
        body_ptr.deref().upcast::<Node>().AppendChild(&DomRoot::upcast(elem)).unwrap();
    }

    pub fn attach(&self, doc: &Document) {
        let e: DomRoot<Element> = doc.CreateElement(DOMString::from_string(self.tag.clone()),
                                                    unsafe { &ElementCreationOptions::empty(doc.window().get_cx()) }).unwrap();
        e.deref().SetId(DOMString::from_string(self.id.clone()));
        Self::append_to_body(doc, e);
        for child in &self.children { child.attach(doc); }
    }

    pub fn update(&self, state: &mut AppState, doc: &Document) {
        for child in &self.children { (child.on_update)(state, doc); }
        (self.on_update)(state, doc);
    }
}

fn frame_callback(doc: &Document) {
    APP_STATE.with(|root| {
        if let Some(state) = root.borrow_mut().as_mut() {
            if !state.model.has_changed { return; }
            let current_observers = state.model.observers.clone();
            for f in current_observers {
                f(doc, state);
            }
            state.model.has_changed = false;
        }
    });
}

pub fn app_main(doc: &Document) {
    ION_APPLICATION_FRAME_CALLBACK.with(|root| root.set(Some(frame_callback)));

    let window = doc.window();
    window.deref().upcast::<EventTarget>().add_event_handler_rust("load", RustEventHandler {
        handler: Rc::new( |doc: &Document, _| {
            let mut state = AppState {
                model: AppStateModel::new(),
                text: HtmlElement::new("p".to_string(), vec![], |state, doc| {
                    let elem_ptr = doc.GetElementById(DOMString::from_string(state.text.id.clone())).unwrap();
                    elem_ptr.deref().SetInnerHTML(DOMString::from_string(format!("The current count is {}!",
                                                                                 state.model.get_count()).to_string())).unwrap();
                }),
                button: HtmlElement::new("button".to_string(), vec![], |state, doc| {
                    let elem_ptr = doc.GetElementById(DOMString::from_string(state.button.id.clone())).unwrap();
                    elem_ptr.deref().SetInnerHTML(DOMString::from_string("Click me!".to_string())).unwrap();
                }),
            };
            state.text.attach(doc);
            state.button.attach(doc);

            let button_ptr = doc.GetElementById(DOMString::from_string(state.button.id.clone())).unwrap();
            let node: &EventTarget = button_ptr.deref().upcast::<EventTarget>();
            node.add_event_handler_rust("click", RustEventHandler {
                handler: Rc::new(|_, _| {
                    APP_STATE.with(|root| {
                        let count = root.borrow().as_ref().unwrap().model.get_count().clone();
                        root.borrow_mut().as_mut().unwrap().model.set_count(count + 1);
                    });
                })
            });

            state.model.on_change(|doc, state: &mut AppState| {
                state.text.clone().update(state, doc);
                state.button.clone().update(state, doc);
            });

            APP_STATE.with(move |root| {
                *root.borrow_mut() = Some(state);
            });
        })
    });
}