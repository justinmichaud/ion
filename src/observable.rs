#[macro_export]
macro_rules! observable {
    ($glob_state_ty:ty, struct $name:ident {
        $($field:ident : $t:ty = $e:expr $(,)*)*
    }) => {
        struct $name {
            $(
                $field : $t,
            )*
            observers: Vec<fn($glob_state_ty, &mut $name)->()>,
            has_changed: bool,
        }

        impl $name {
            pub fn new() -> $name {
                $name {
                    $(
                        $field : $e,
                    )*
                    observers: vec![],
                    has_changed: true,
                 }
            }

            pub fn on_change(&mut self, observer: fn($glob_state_ty, &mut $name)->()) {
                self.observers.push(observer)
            }

            pub fn tick(&mut self, doc: $glob_state_ty) {
                if !self.has_changed { return; }
                let current_observers = self.observers.clone();
                for f in current_observers {
                    f(doc, self);
                }
                self.has_changed = false;
            }

            $(
                interpolate_idents! {
                    fn [get_ $field](&self) -> $t { self.$field }
                    fn [set_ $field](&mut self, val: $t) {
                        self.$field = val;
                        self.has_changed = true;
                    }
                }
            )*
        }
    }
}