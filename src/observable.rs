#[macro_export]
macro_rules! observable {
    ($glob_state_ty:ty, $app_state_ty:ty, struct $name:ident {
        $($field:ident : $t:ty = $e:expr $(,)*)*
    }) => {
        struct $name {
            $(
                $field : $t,
            )*
            observers: Vec<fn($glob_state_ty, $app_state_ty)->()>,
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

            pub fn on_change(&mut self, observer: fn($glob_state_ty, $app_state_ty)->()) {
                self.observers.push(observer)
            }

            $(
                interpolate_idents! {
                    #[allow(dead_code)]
                    fn [get_ $field](&self) -> &$t { &self.$field }
                    #[allow(dead_code)]
                    fn [get_ $field _mut](&mut self) -> &mut $t { &mut self.$field }
                    #[allow(dead_code)]
                    fn [set_ $field](&mut self, val: $t) {
                        self.$field = val;
                        self.has_changed = true;
                    }
                }
            )*
        }
    }
}