use std::rc::Rc;
use std::cell::RefCell;
use html::HtmlElement;
use html::RustEventHandler;

make_app_setup!{ pub fn app_setup() app_thread_state = APP_STATE, render = render }
thread_local!(static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new()));

struct TodoItem {
    id: u32,
    name: String,
    editing: bool,
}

observable! {struct AppState {
    items : Vec<TodoItem> = vec![TodoItem {id: 1, name: "Testing!".to_string(), editing: false}],
    new_item_name: String = "Item Name".to_string(),
}}

fn render_item(item: &TodoItem) -> HtmlElement {
    let item_id = item.id.clone();

    HtmlElement::new(None as Option<String>, "div", "", "", "",   hashmap!(),vec![
        if item.editing {
            render_edit(item)
        } else {
            HtmlElement::new(None as Option<String>, "p", item.name.clone(), "", "",   hashmap!(),vec![])
        },
        HtmlElement::new(None as Option<String>, "button", if item.editing { "Save" } else { "Edit" }, "", "", hashmap!("click".to_string() => RustEventHandler {
            handler: Rc::new(move |_, _| {
                APP_STATE.with(|root| {
                    let mut state = root.borrow_mut();
                    for i in state.get_items_mut() {
                        if i.id == item_id {
                            i.editing = !i.editing;
                        }
                    }
                });
            })
        }), vec![])
    ])
}

fn render_edit(item: &TodoItem) -> HtmlElement {
    let mut input = HtmlElement::new(Some(format!("item_{}", item.id)), "textarea", item.name.clone(), "", "", hashmap!(), vec![]);
    let input_id = input.get_id().clone();
    let item_id = item.id.clone();
    input.add_listener(vec!["input", "keyup"], RustEventHandler {
        handler: Rc::new(move |doc, _| {
            APP_STATE.with(|root| {
                let mut state = root.borrow_mut();
                for i in state.get_items_mut() {
                    if i.id == item_id {
                        i.name = HtmlElement::get_dom_element_value(&input_id, doc);
                    }
                }
            });
        })
    });

    input
}

fn render_add(state: &AppState) -> HtmlElement {
    let mut input = HtmlElement::new(Some("add_input"), "textarea", state.get_new_item_name(), "", "", hashmap!(), vec![]);
    let input_id = input.get_id().clone();
    input.add_listener(vec!["input", "keyup"], RustEventHandler {
        handler: Rc::new(move |doc, _| {
            APP_STATE.with(|root| {
                let mut state = root.borrow_mut();
                state.set_new_item_name(HtmlElement::get_dom_element_value(&input_id, doc));
            });
        })
    });

    HtmlElement::new(None as Option<String>, "div", "", "", "", hashmap!(), vec![
        HtmlElement::new(None as Option<String>, "h3", "Add item", "", "", hashmap!(), vec![]),

        input,

        HtmlElement::new(None as Option<String>, "button", "+", "", "", hashmap!("click".to_string() => RustEventHandler {
            handler: Rc::new(move |_, _| {
                APP_STATE.with(|root| {
                    let mut state = root.borrow_mut();
                    {
                        let name = state.get_new_item_name().clone();
                        let items = state.get_items_mut();
                        let mut max_id = 0;
                        for item in &*items { if item.id > max_id { max_id = item.id; } }

                        items.push(TodoItem { id: max_id + 1, name, editing: false });
                    }
                    state.set_new_item_name("Item Name".to_string());
                });
            })
        }), vec![]),
    ])
}

fn render(state: &AppState) -> HtmlElement {
    HtmlElement::new(None as Option<String>, "div", "", "", "", hashmap!(), vec![
        HtmlElement::new(None as Option<String>, "h1", "Todo List", "", "", hashmap!(), vec![]),

        HtmlElement::new(None as Option<String>, "div", "", "", "", hashmap!(),
                         state.get_items().iter().map(render_item).collect()),

        render_add(state),
    ])
}
