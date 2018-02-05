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

pub fn app_main(doc: &Document) {
    let window = doc.window();
    window.deref().upcast::<EventTarget>().add_event_handler_rust("load", RustEventHandler {
        handler: Rc::new( |doc, cx| {
            use std::cell::RefCell;
            thread_local!(static COUNT: RefCell<i32> = RefCell::new(0));

            let body_collection = doc.GetElementsByTagName(DOMString::from_string("body".to_string()));
            let body_ptr = body_collection.elements_iter().last().unwrap();

            let button: DomRoot<Element> = doc.CreateElement(DOMString::from_string("button".to_string()),
                                                             unsafe { &ElementCreationOptions::empty(cx) }).unwrap();
            button.deref().SetInnerHTML(DOMString::from_string("Click me!".to_string())).unwrap();
            body_ptr.deref().upcast::<Node>().AppendChild(&DomRoot::upcast(button)).unwrap();

            let text: DomRoot<Element> = doc.CreateElement(DOMString::from_string("p".to_string()),
                                                           unsafe { &ElementCreationOptions::empty(cx) }).unwrap();
            text.deref().SetInnerHTML(DOMString::from_string("The current count is 0!".to_string())).unwrap();
            text.deref().SetId(DOMString::from_string("myid".to_string()));
            body_ptr.deref().upcast::<Node>().AppendChild(&DomRoot::upcast(text)).unwrap();

            let node: &EventTarget = body_ptr .deref().upcast::<EventTarget>();
            node.add_event_handler_rust("click", RustEventHandler {
                handler: Rc::new(|doc,_| {
                    COUNT.with(|root| *root.borrow_mut() += 1);
                    COUNT.with(|root| {
                        let elem_ptr = doc.GetElementById(DOMString::from_string("myid".to_string())).unwrap();
                        elem_ptr.deref().SetInnerHTML(DOMString::from_string(format!("The current count is {}!",
                                                                                     *root.borrow()).to_string())).unwrap();
                    });
                })
            });
        })
    });
}