use std::rc::Rc;
use std::cell::RefCell;
use html::HtmlElement;
use html::RustEventHandler;

make_app_setup!{ pub fn app_setup() app_thread_state = APP_STATE, render = render }
thread_local!(static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new()));

struct TodoItem {
    id: u32,
    name: String,
    done: bool,
}

observable! {struct AppState {
    items : Vec<TodoItem> = vec![TodoItem {id: 1, name: "Testing!".to_string(), done: true}],
    new_item_name: String = "Item Name".to_string(),
}}

fn render_item(item: &TodoItem) -> HtmlElement {
    HtmlElement::new(Some(format!("item_{}", item.id)), "div", "", "", "",   hashmap!(),vec![
        HtmlElement::new(None as Option<String>, "p", item.name.clone(), "", "",   hashmap!(),vec![])
    ])
}

fn render_add(state: &AppState) -> HtmlElement {
    let mut input = HtmlElement::new(Some("add_input"), "textarea", state.get_new_item_name(), "", "", hashmap!(), vec![]);
    let input_id = input.get_id().clone();
    let input_id2 = input_id.clone();
    input.add_listener("click", RustEventHandler {
        handler: Rc::new(move |doc, _| {
            APP_STATE.with(|root| {
                let mut state = root.borrow_mut();
                state.set_new_item_name(HtmlElement::get_dom_element_value(&input_id, doc));
            });
        })
    });

    HtmlElement::new(None as Option<String>, "div", "", "", "", hashmap!(), vec![
        input,

        HtmlElement::new(None as Option<String>, "button", "+", "", "", hashmap!("click".to_string() => RustEventHandler {
            handler: Rc::new(move |doc, _| {
                APP_STATE.with(|root| {
                    let mut state = root.borrow_mut();
                    {
                        let items = state.get_items_mut();
                        let mut max_id = 0;
                        for item in &*items { if item.id > max_id { max_id = item.id; } }

                        let value = HtmlElement::get_dom_element_value(&input_id2, doc);
                        items.push(TodoItem { id: max_id + 1, name: value, done: false });
                    }
                    state.set_new_item_name("Item Name".to_string());
                });
            })
        }), vec![]),
    ])
}

fn render(state: &AppState) -> HtmlElement {
    HtmlElement::new(None as Option<String>, "div", "", "", "", hashmap!(), vec![
        HtmlElement::new(None as Option<String>, "div", "", "", "", hashmap!(),
                         state.get_items().iter().map(render_item).collect()),

        render_add(state),
    ])
}
