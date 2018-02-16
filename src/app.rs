use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std;

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

struct TodoItem {
    name: String,
    done: bool,
}

observable! {struct AppState {
    items : Vec<TodoItem> = vec![TodoItem {name: "Testing!".to_string(), done: true}],
}}

type HtmlElementRender = Box<Fn(u32, &mut AppState, &mut AppView, &Document)->()>;

struct HtmlElement {
    id: String,
    tag: String,
    render: HtmlElementRender,
}

impl fmt::Debug for HtmlElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.id.clone(), self.tag.clone()).fmt(f)
    }
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
            *root.borrow_mut() = val;
            val
        })
    }

    pub fn make_child<T>(&mut self, parent: Option<u32>, tag: T, render: HtmlElementRender) -> u32
            where T: ToString {
        let id = Self::gen_id();
        let e = HtmlElement { id: id.to_string(), tag: tag.to_string(), render };

        self.elements.insert(id, e);

        if let Some(p) = parent {
            self.parent_to_children.entry(p).or_insert(vec![]).push(id);
            self.child_to_parent.insert(id, p);
        } else {
            self.roots.push(id);
        }
        
        id
    }

    pub fn children(&self, id: u32) -> Vec<u32> {
        if let Some(children) = self.parent_to_children.get(&id) {
            children.clone()
        } else { vec![] }
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

    fn detach(&mut self, id: u32, doc: &Document) {
        if let Some(elem_ptr) = doc.GetElementById(ds(id)) {
            elem_ptr.deref().Remove();
        }
        self.elements.remove(&id);
        self.child_to_parent.remove(&id);
    }

    pub fn detach_children(&mut self, id: u32, doc: &Document) {
        println!("Detaching children of {}", id);
        for child in self.children(id) {
            self.detach(child, doc);
        }
        self.parent_to_children.remove(&id);
    }

    fn render_helper(&mut self, doc: &Document, id: u32) {
        println!("Rendering {}", id);
        let mut render = if let Some(ref mut e) = self.elements.get_mut(&id) {
            std::mem::replace(&mut e.render, Box::new(|_,_,_,_| {}))
        } else { return };

        APP_STATE.with(|root| render(id, &mut *root.borrow_mut(), self, doc));

        if let Some(ref mut e) = self.elements.get_mut(&id) {
            std::mem::swap(&mut render,&mut e.render)
        }

        for child in self.children(id) {
            self.render_helper(doc, child);
        }
    }

    pub fn render(&mut self, doc: &Document) {
        println!("Rendering: {:?}", self.elements);
        let roots = self.roots.clone();

        for root in roots {
            self.render_helper(doc, root);
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

fn frame_callback(doc: &Document) {
    let has_changed = APP_STATE.with(|root| {
        let val = root.borrow().has_changed;
        root.borrow_mut().has_changed = false;
        val
    });
    if !has_changed { return };
    APP_VIEWS.with(|root| root.borrow_mut().render(doc) );
}

pub fn app_main(doc: &Document) {
    let button = APP_VIEWS.with(|root| {
        root.borrow_mut().make_child(None, "div", Box::new(|id, state: &mut AppState, view: &mut AppView, doc| {
            view.detach_children(id, doc);

            for item in state.get_items() {
                let name = item.name.clone();
                let cid = view.make_child(Some(id), "div", Box::new(  move|id, _: &mut AppState, view: &mut AppView, doc| {
                    let name2 = (&name).clone();
                    //view.detach_children(id, doc);
                    let cid = view.make_child(Some(id), "p", Box::new(  move |id,_,_,doc| {
                        set_text(id, doc, (&name2).clone())
                    }));
                    view.attach(cid, doc);
                }));
                view.attach(cid, doc);
            }
        }));

        root.borrow_mut().make_child(None, "button", Box::new(|id, _, _, doc| {
            set_text(id, doc, "Add new entry!")
        }))
    });

    let button_click = RustEventHandler {
        handler: Rc::new(|_, _| {
            APP_STATE.with(|root| {
                let mut state = root.borrow_mut();
                let items = state.get_items_mut();
                let len = items.len();
                println!("Adding {}", len);
                items.push(TodoItem {name: format!("Pushed {}", len).to_string(), done: true});
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

            set_event(button, doc, "click", button_click.clone());

            ION_APPLICATION_FRAME_CALLBACK.with(|root| {
                assert!(root.get().is_none());
                root.set(Some(frame_callback))
            });
        })
    });
}