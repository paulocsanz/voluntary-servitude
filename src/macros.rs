//! Contains all crate macros

/// Alias for [`voluntary_servitude`] macro
///
/// [`voluntary_servitude`]: ./macro.voluntary_servitude.html
///
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// use voluntary_servitude::VS;
/// let vs: VS<()> = vs![];
/// assert!(vs.is_empty());
///
/// let vs = vs![1, 2, 3];
/// assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1, &2, &3]);
///
/// let vs = vs![1; 3];
/// assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1; 3]);
/// ```
#[macro_export]
macro_rules! vs {
    () => (voluntary_servitude![]);
    ($elem: expr; $n: expr) => (voluntary_servitude![$elem; $n]);
    ($($x: expr),*) => (voluntary_servitude![$($x),*]);
}

/// Creates new [`VS`] with specified elements as in the `vec!` macro
///
/// [`VS`]: ./type.VS.html
///
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// use voluntary_servitude::VS;
/// let vs: VS<()> = voluntary_servitude![];
/// assert!(vs.is_empty());
///
/// let vs = voluntary_servitude![1, 2, 3];
/// assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1, &2, &3]);
///
/// let vs = voluntary_servitude![1; 3];
/// assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1; 3]);
/// ```
#[macro_export]
macro_rules! voluntary_servitude {
    () => ($crate::VS::default());
    ($elem: expr; $n: expr) => {{
        let vs = $crate::VS::default();
        let _ = (0..$n).map(|_| vs.append($elem)).count();
        vs
    }};
    ($($x: expr),*) => {{
        let vs = $crate::VS::default();
        $(vs.append($x);)*
        vs
    }};
}

/// Used to tell the compiler this branch is never taken (panics in debug, unreachable in release)
macro_rules! never {
    ($($x: expr),*) => {{
        #[cfg(debug_assertions)]
        panic!($($x,)*);

        #[cfg(not(debug_assertions))]
        #[allow(unused_unsafe)]
        unsafe { ::std::hint::unreachable_unchecked() };
    }}
}

/// Used to tell the compiler `Result` is never `Err` (panics in debug, unreachable in release)
macro_rules! success {
    ($res: expr, $($x: expr),*) => ($res.map_err(|_| never!($($x),*)));
}
