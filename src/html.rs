use std::ops::Deref;
use std::rc::Rc;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use servo::script::dom::document::Document;
use servo::script::dom::bindings::str::DOMString;
use servo::script::dom::eventtarget::EventTarget;
use servo::script::dom::bindings::codegen::Bindings::DocumentBinding::DocumentMethods;
use servo::script::dom::bindings::codegen::Bindings::ElementBinding::ElementMethods;
use servo::script::dom::bindings::inheritance::Castable;
use servo::script::dom::node::Node;
use servo::script::dom::bindings::codegen::Bindings::NodeBinding::NodeMethods;
use servo::script::dom::bindings::root::DomRoot;
use servo::script::dom::bindings::codegen::Bindings::DocumentBinding::ElementCreationOptions;
use servo::script::dom::element::Element;
use servo::script::script_thread::ION_APPLICATION_FRAME_CALLBACK;

pub use servo::script::dom::eventtarget::RustEventHandler;

thread_local!(pub static RENDER: Cell<Option<fn()->Option<HtmlElement>>> = Cell::new(None));

fn ds<T>(str: T) -> DOMString where T: ToString { DOMString::from_string(str.to_string()) }

pub struct HtmlElement {
    id: String,
    tag: String,
    text: String,
    listeners: HashMap<String, RustEventHandler>,
    children: Vec<HtmlElement>
}

impl fmt::Debug for HtmlElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.id.clone(), self.tag.clone()).fmt(f)
    }
}

impl HtmlElement {
    fn gen_id() -> u32 {
        thread_local!(static ID_COUNTER: RefCell<u32> = RefCell::new(0));
        ID_COUNTER.with(|root| {
            let val = *root.borrow() + 1;
            *root.borrow_mut() = val;
            val
        })
    }

    pub fn get_dom_element_value(id: &String, doc: &Document) -> String {
        let elem_ptr = doc.GetElementById(ds(id)).unwrap();
//        elem_ptr.deref().GetAttribute(ds("value")).unwrap().to_string()
        elem_ptr.deref().GetInnerHTML().unwrap().to_string()
    }

    pub fn new<T: ToString, U: ToString, V: ToString>(unique_key: Option<T>, tag: U, text: V,
                                                      listeners: HashMap<String, RustEventHandler>,
                                                      children: Vec<HtmlElement>) -> HtmlElement {
        let id = match unique_key {
            Some(k) => format!("unique_key_{}", k.to_string()),
            _ => Self::gen_id().to_string(),
        };
        HtmlElement {
            id,
            tag: tag.to_string(),
            text: text.to_string(),
            listeners,
            children,
        }
    }

    fn render_to_dom_as_root(&self, doc: &Document) {
        let body_collection = doc.GetElementsByTagName(ds("body"));
        let body_ptr = body_collection.elements_iter().last().unwrap();
        {
            let body_node: &Node = body_ptr.deref().upcast::<Node>();
            let new_node = &DomRoot::upcast(self.make_tree(doc));

            match body_node.GetFirstChild() {
                Some(ref child) => body_node.ReplaceChild(new_node, child).unwrap(),
                None => body_node.AppendChild(new_node).unwrap(),
            };
        }
    }

    fn make_tree(&self, doc: &Document) -> DomRoot<Element> {
        // TODO: IF doc already has element with this id, then detach and produce that
        let dom_elem: DomRoot<Element> = doc.CreateElement(DOMString::from_string(self.tag.clone()),
                                                           unsafe { &ElementCreationOptions::empty(doc.window().get_cx()) }).unwrap();
        dom_elem.deref().SetId(ds(self.id.clone()));
        dom_elem.deref().SetInnerHTML(ds(self.text.clone())).unwrap(); //TODO: SetTextContent
        for (event, listener) in &self.listeners {
            let node: &EventTarget = dom_elem.upcast::<EventTarget>();
            node.add_event_handler_rust(ds(event), listener.clone());
        }
        for child in &self.children {
            let dom_child = child.make_tree(doc);
            dom_elem.upcast::<Node>().AppendChild(&DomRoot::upcast(dom_child)).unwrap();
        }
        dom_elem
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }
    pub fn add_listener<T: ToString>(&mut self, event: T, listener: RustEventHandler) {
        self.listeners.insert(event.to_string(), listener);
    }
}

fn frame_callback(doc: &Document) {
    RENDER.with(|root| match (root.get().expect("Frame callback should not be set before html::RENDER is"))() {
        Some(elem) => elem.render_to_dom_as_root(doc),
        None => {}
    });
}

pub fn app_main(doc: &Document) {
    let window = doc.window();
    window.deref().upcast::<EventTarget>().add_event_handler_rust(ds("load"), RustEventHandler {
        handler: Rc::new(  |_, _| {
            ION_APPLICATION_FRAME_CALLBACK.with(|root| {
                assert!(root.get().is_none());
                root.set(Some(frame_callback))
            });
        })
    });
}

#[macro_export]
macro_rules! make_app_setup {
    (pub fn $app_setup_name:ident() app_thread_state = $app_state_thread_local_name:ident, render = $render:ident) => {
        fn _do_not_use_make_app_setup_twice_in_one_file() -> Option<HtmlElement> {
            let has_changed = $app_state_thread_local_name.with(|root| {
                let val = root.borrow().has_changed;
                root.borrow_mut().has_changed = false;
                val
            });
            if !has_changed { return None };
            Some($app_state_thread_local_name.with(|state| $render(&*state.borrow())))
        }

        pub fn $app_setup_name() {
            use html::RENDER;
            RENDER.with(|root| {
                assert!(root.get().is_none());
                root.set(Some(_do_not_use_make_app_setup_twice_in_one_file))
            });
        }
    }
}