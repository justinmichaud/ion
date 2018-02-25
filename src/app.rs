use std::rc::Rc;
use std::cell::RefCell;
use html::HtmlElement;
use html::RustEventHandler;

make_app_setup!{ pub fn app_setup() app_thread_state = APP_STATE, render = render }
thread_local!(static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new()));

observable! {struct AppState {
    text: String = "ENTER TEXT".to_string()
}}

fn render(state: &AppState) -> HtmlElement {
    println!("Rendering with text {}", state.get_text());
    let mut tx = HtmlElement::new(Some("id"), "textarea", state.get_text(), hashmap!(), vec![]);
    let id = tx.get_id();
    tx.add_listener("input", RustEventHandler {
        handler: Rc::new(move |doc, _| {
            let text = HtmlElement::get_dom_element_value(&id, doc).to_uppercase();
            println!("Got text {}", text);

            APP_STATE.with(|root| {
                let mut state = root.borrow_mut();
                if *state.get_text() != text {
                    println!("Set text to {}", text);
                    *state.get_text_mut() = text.clone();
                }
            });
        })
    });
    tx
}
