//! Static site generator for «Карта изучения православия»: reads `content/`, checks it,
//! lays the topics out as a tree and renders a static site for GitHub Pages.

pub mod layout;
pub mod markdown;
pub mod model;
pub mod render;
pub mod serve;
pub mod validate;

pub const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("OMAP_GIT_SHA"),
    ", built ",
    env!("OMAP_BUILT"),
    ")"
);
