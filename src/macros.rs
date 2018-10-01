//! Contains all crate macros

/// Creates new [`VS`] with specified elements as in the 'vec!' macro
///
/// [`VS`]: ./type.VS.html
///
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// # use voluntary_servitude::VS;
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

/// Creates new [`VS`] with specified elements as in the 'vec!' macro
///
/// [`VS`]: ./type.VS.html
///
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// use voluntary_servitude::VS;
/// let vs: VS<()> = vs![];
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
        let voluntary_servitude = $crate::VS::default();
        let _ = (0..$n).map(|_| voluntary_servitude.append($elem)).count();
        voluntary_servitude
    }};
    ($($x: expr),*) => ({
        let voluntary_servitude = $crate::VS::default();
        $(voluntary_servitude.append($x);)*
        voluntary_servitude
    });
}

/// Executes expression and returns true (useful to conditionally execute a statement with '.filter')
///
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// let mut opt = Some(1);
/// let mut count = 0;
/// assert!(opt.filter(|_| truth!(count += 1)).is_some());
/// assert_eq!(count, 1);
///
/// opt = None;
/// assert!(opt.filter(|_| truth!(count += 1)).is_none());
/// assert_eq!(count, 1);
/// ```
macro_rules! truth {
    ($expr: expr) => {{
        $expr;
        true
    }};
}

/// Returns specified value if pointer is null (defaults to ())
///
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// use std::ptr::null;
/// let f = |ptr| {
///     null_check!(ptr, true);
///     false
/// };
/// assert!(f(null()));
/// assert!(!f(&f as *const _));
/// ```
macro_rules! null_check {
    ($ptr: expr, $ret: expr) => {{
        if $ptr.is_null() {
            return $ret;
        }
    }};
    ($ptr: expr) => {
        null_check!($prt, ())
    };
}
