/// Creates new VSRead with specified elements as in the 'vec!' macro
///
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// let vsread = vsread![1, 2, 3];
/// assert_eq!(vsread.iter().collect::<Vec<_>>(), vec![&1, &2, &3]);
///
/// let vsread = vsread![1; 3];
/// assert_eq!(vsread.iter().collect::<Vec<_>>(), vec![&1, &1, &1]);
/// ```
#[macro_export]
macro_rules! vsread {
    () => ($crate::VSRead::default());
    ($elem: expr; $n: expr) => {{
        let vsread = $crate::VSRead::default();
        let _ = (0..$n).map(|_| vsread.append($elem)).count();
        vsread
    }};
    ($($x: expr),*) => ({
        let vsread = $crate::VSRead::default();
        $(
            vsread.append($x);
        )*
        vsread
    });
    ($($x: expr,)*) => (vsread![$($x),*]);
}

/// Returns specified value if pointer is null, if not apply manipulation
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// use std::ptr::null;
/// let test = |ptr| non_null!(ptr, *ptr, 0);
/// assert_eq!(test(null()), 0);
/// assert_eq!(test(&3 as *const i32), 3i32);
/// ```
macro_rules! non_null {
    ($expr: expr, $op: expr, $value: expr) => {{
        if $expr.is_null() {
            return $value;
        }
        $op
    }}
}
