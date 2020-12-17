
pub mod probe;

#[cfg(feature = "tokio_executor")]
#[macro_export]
macro_rules! test_fn {
    ($(#[$($meta:meta)*])* $vis:vis $ident:ident $($tokens:tt)*) => {
        $(#[$($meta)*])* #[tokio::test] $vis async $ident $($tokens)*
    };
}
#[cfg(not(feature = "tokio_executor"))]
#[macro_export]
macro_rules! test_fn {
    ($(#[$($meta:meta)*])* $vis:vis $ident:ident $($tokens:tt)*) => {
        $(#[$($meta)*])* #[test] $vis $ident $($tokens)*
    };
}
