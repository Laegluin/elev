extern crate embed_resource;

use std::env;

fn main() {
    if env::var_os("CARGO_FEATURE_REQUIRE_ELEVATION").is_some() {
        embed_resource::compile("resources.rc");
    }
}
