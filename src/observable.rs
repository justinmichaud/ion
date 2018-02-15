#[macro_export]
macro_rules! observable {
    (struct $name:ident {
        $($field:ident : $t:ty = $e:expr $(,)*)*
    }) => {
        struct $name {
            $(
                $field : $t,
            )*
            has_changed: bool,
        }

        impl $name {
            pub fn new() -> $name {
                $name {
                    $(
                        $field : $e,
                    )*
                    has_changed: true,
                 }
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