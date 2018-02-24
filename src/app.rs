use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

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

fn ds<T>(str: T) -> DOMString where T: ToString { DOMString::from_string(str.to_string()) }

struct HtmlElement {
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

    pub fn render_to_dom_as_root(&self, doc: &Document) {
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
}

thread_local!(static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new()));

struct TodoItem {
    name: String,
    done: bool,
}

observable! {struct AppState {
    items : Vec<TodoItem> = vec![TodoItem {name: "Testing!".to_string(), done: true}],
}}

fn frame_callback(doc: &Document) {
    let has_changed = APP_STATE.with(|root| {
        let val = root.borrow().has_changed;
        root.borrow_mut().has_changed = false;
        val
    });
    if !has_changed { return };
    APP_STATE.with(|state| render(&*state.borrow()).render_to_dom_as_root(doc));
}

fn render(state: &AppState) -> HtmlElement {
    HtmlElement::new(None as Option<String>, "div", "", hashmap!(), state.get_items().iter().map(|item: &TodoItem|
        HtmlElement::new(None as Option<String>, "p", item.name.clone(),  hashmap!(),vec![])
    ).chain(vec![
        HtmlElement::new(None as Option<String>, "button", "Click Me!", hashmap!("click".to_string() => RustEventHandler {
            handler: Rc::new(|_, _| {
                APP_STATE.with(|root| {
                    let mut state = root.borrow_mut();
                    let items = state.get_items_mut();
                    let len = items.len();
                    println!("Adding {}", len);
                    items.push(TodoItem {name: format!("Pushed {}", len).to_string(), done: true});
                });
            })
        }), vec![]),
    ]).collect())
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