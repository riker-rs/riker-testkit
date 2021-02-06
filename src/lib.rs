pub use test_fn_macro::test;
pub mod probe;

#[macro_export]
macro_rules! test_fn {
    ($(#[$($meta:meta)*])* $vis:vis $ident:ident $($tokens:tt)*) => {
        $(#[$($meta)*])* #[tokio::test] $vis async $ident $($tokens)*
    };
}
