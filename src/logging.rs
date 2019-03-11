#[allow(unused_macros)]
macro_rules! debug {
    ( $( $e:expr ),* $(,)* ) => {
        if false {
            $(
                let _ = $e;
            )*
        }
    }
}

#[allow(unused_macros)]
macro_rules! error {
    ( $( $e:expr ),* $(,)* ) => {
        if false {
            $(
                let _ = $e;
            )*
        }
    }
}

#[allow(unused_macros)]
macro_rules! info {
    ( $( $e:expr ),* $(,)* ) => {
        if false {
            $(
                let _ = $e;
            )*
        }
    }
}

#[allow(unused_macros)]
macro_rules! log {
    ( $( $e:expr ),* $(,)* ) => {
        if false {
            $(
                let _ = $e;
            )*
        }
    }
}

#[allow(unused_macros)]
macro_rules! trace {
    ( $( $e:expr ),* $(,)* ) => {
        if false {
            $(
                let _ = $e;
            )*
        }
    }
}

#[allow(unused_macros)]
macro_rules! warn {
    ( $( $e:expr ),* $(,)* ) => {
        if false {
            $(
                let _ = $e;
            )*
        }
    }
}
