#[cfg(feature = "profiling_spans")]
macro_rules! px_trace_span {
    ($($arg:tt)*) => {
        bevy_log::tracing::trace_span!($($arg)*).entered()
    };
}
#[cfg(not(feature = "profiling_spans"))]
macro_rules! px_trace_span {
    ($($arg:tt)*) => {
        ()
    };
}

#[cfg(feature = "profiling_spans")]
macro_rules! px_trace {
    ($($arg:tt)*) => {
        bevy_log::tracing::trace!($($arg)*)
    };
}
#[cfg(not(feature = "profiling_spans"))]
macro_rules! px_trace {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "profiling_spans")]
macro_rules! px_end_span {
    ($span:ident) => {
        drop($span);
    };
}
#[cfg(not(feature = "profiling_spans"))]
macro_rules! px_end_span {
    ($span:ident) => {};
}

#[cfg(feature = "profiling_spans")]
macro_rules! px_profile {
    (let mut $name:ident = $value:expr) => {
        let mut $name = $value;
    };
    ($name:ident += $value:expr) => {
        $name += $value;
    };
    (emit $($arg:tt)*) => {
        bevy_log::tracing::trace!($($arg)*);
    };
}
#[cfg(not(feature = "profiling_spans"))]
macro_rules! px_profile {
    (let mut $name:ident = $value:expr) => {};
    ($name:ident += $value:expr) => {};
    (emit $($arg:tt)*) => {};
}

pub(crate) use px_end_span;
pub(crate) use px_profile;
pub(crate) use px_trace;
pub(crate) use px_trace_span;
