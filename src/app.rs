use std::rc::Rc;
use std::cell::RefCell;

use html::HtmlElement;
use html::RustEventHandler;

make_app_setup!{ pub fn app_setup() app_thread_state = APP_STATE, render = render }
thread_local!(static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new()));

struct TodoItem {
    name: String,
}

observable! {struct AppState {
    items : Vec<TodoItem> = vec![TodoItem {name: "Testing!".to_string(), done: true}],
}}

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
