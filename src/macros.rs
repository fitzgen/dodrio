macro_rules! debug {
    ( $fmt:expr $(, $x:expr )* $(,)* ) => {{
        if cfg!(debug_assertions) {
            web_sys::console::log_1(&format!($fmt, $($x),*).into());
        }
    }}
}
