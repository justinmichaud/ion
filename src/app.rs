use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

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

thread_local!(static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new()));
thread_local!(static APP_VIEWS: RefCell<AppView> = RefCell::new(AppView::default()));

observable! {struct AppState {
    count : i32 = 0,
}}

type HtmlElementRender = fn(id: u32, &mut AppState, &mut AppView, &Document)->();

struct HtmlElement {
    id: String,
    tag: String,
    render: HtmlElementRender,
}

#[derive(Default)]
struct AppView {
    elements: HashMap<u32, HtmlElement>,
    parent_to_children: HashMap<u32, Vec<u32>>,
    child_to_parent: HashMap<u32, u32>,
    roots: Vec<u32>,
}

impl AppView {
    fn gen_id() -> u32 {
        thread_local!(static ID_COUNTER: RefCell<u32> = RefCell::new(0));
        ID_COUNTER.with(|root| {
            let val = *root.borrow() + 1;
            *root.borrow_mut() += val;
            val
        })
    }

    pub fn make_child<T>(&mut self, parent: Option<u32>, tag: T, render: HtmlElementRender) -> u32
            where T: ToString {
        let id = Self::gen_id();
        let e = HtmlElement {  id: id.to_string(), tag: tag.to_string(), render };

        self.elements.insert(id, e);

        if let Some(p) = parent {
            self.parent_to_children.entry(p).or_insert(vec![]).push(id);
            self.child_to_parent.insert(id, p);
        } else {
            self.roots.push(id);
        }
        
        id
    }

    pub fn attach_all(&self, doc: &Document) {
        for root in &self.roots {
            self.attach(*root, doc);
        }
    }

    pub fn attach(&self, id: u32, doc: &Document) {
        let elem = &self.elements[&id];
        let dom_emem: DomRoot<Element> = doc.CreateElement(DOMString::from_string(elem.tag.clone()),
                                                           unsafe { &ElementCreationOptions::empty(doc.window().get_cx()) }).unwrap();
        dom_emem.deref().SetId(DOMString::from_string(elem.id.clone()));
        match self.child_to_parent.get(&id) {
            Some(p) => {
                let parent_ptr = doc.GetElementById(ds(p)).unwrap();
                parent_ptr.deref().upcast::<Node>().AppendChild(&DomRoot::upcast(dom_emem)).unwrap();
            }
            None => {
                let body_collection = doc.GetElementsByTagName(ds("body"));
                let body_ptr = body_collection.elements_iter().last().unwrap();
                body_ptr.deref().upcast::<Node>().AppendChild(&DomRoot::upcast(dom_emem)).unwrap();
            }
        }

        if let Some(children) = self.parent_to_children.get(&id) {
            for child in children {
                self.attach(*child, doc);
            }
        }
    }

    fn render_helper(&mut self, doc: &Document, id: u32, p2c: &HashMap<u32, Vec<u32>>) {
        let render = if let Some(e) = self.elements.get(&id) {
            e.render.clone()
        } else { return };

        APP_STATE.with(|root| render(id, &mut *root.borrow_mut(), self, doc));

        if let Some(children) = p2c.get(&id) {
            for child in children {
                self.render_helper(doc, *child, p2c);
            }
        }
    }

    pub fn render(&mut self, doc: &Document) {
        let p2c = self.parent_to_children.clone();
        let roots = self.roots.clone();

        for root in roots {
            self.render_helper(doc, root, &p2c);
        }
    }
}

fn set_text<T>(id: u32, doc: &Document, text: T) where T: ToString {
    let elem_ptr = doc.GetElementById(ds(id)).unwrap();
    elem_ptr.deref().SetInnerHTML(ds(text)).unwrap();
}

fn set_attribute<T,V>(id: u32, doc: &Document, prop: T, val: V) where T: ToString, V: ToString {
    let elem_ptr = doc.GetElementById(ds(id)).unwrap();
    elem_ptr.deref().SetAttribute(ds(prop), ds(val)).unwrap();
}

fn set_event<T>(id: u32, doc: &Document, event: T, handler: RustEventHandler) where T: ToString {
    let elem_ptr = doc.GetElementById(ds(id)).unwrap();
    let node: &EventTarget = elem_ptr.deref().upcast::<EventTarget>();
    node.add_event_handler_rust(ds(event), handler);
}

fn frame_callback(doc: &Document) { //TODO: If this always modifies the DOM, the intermediate state is never rendered
    let has_changed = APP_STATE.with(|root| {
        let val = root.borrow().has_changed;
        root.borrow_mut().has_changed = false;
        val
    });
    if !has_changed { return };
    APP_VIEWS.with(|root| root.borrow_mut().render(doc) );
}

pub fn app_main(doc: &Document) {
    ION_APPLICATION_FRAME_CALLBACK.with(|root| {
        assert!(root.get().is_none());
        root.set(Some(frame_callback))
    });

    let button = APP_VIEWS.with(|root| {
        let parent = root.borrow_mut().make_child(None, "div", |id, _, _, doc| {
            set_attribute(id, doc,"style", "background: #eee")
        });
        root.borrow_mut().make_child(Some(parent), "p".to_string(), |id, state, _, doc| {
            set_text(id, doc,format!("The current count is {}!", state.get_count()))
        });
        root.borrow_mut().make_child(Some(parent), "button", |id, _, _, doc| {
            set_text(id, doc, "Click Me!")
        })
    });

    let button_click = RustEventHandler {
        handler: Rc::new(|_, _| {
            APP_STATE.with(|root| {
                let count = root.borrow().get_count().clone();
                root.borrow_mut().set_count(count + 1);
            });
        })
    };

    let window = doc.window();
    window.deref().upcast::<EventTarget>().add_event_handler_rust(ds("load"), RustEventHandler {
        handler: Rc::new( move |doc: &Document, _| {
            APP_VIEWS.with(|root| {
                root.borrow_mut().attach_all(doc);
                root.borrow_mut().render(doc);
            });

            set_event(button,doc, "click", button_click.clone());
        })
    });
}