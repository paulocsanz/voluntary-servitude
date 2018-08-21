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
