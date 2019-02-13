use embed_resource;

use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=ELEV_RUN_SHA256");

    if env::var_os("CARGO_FEATURE_REQUIRE_ELEVATION").is_some() {
        embed_resource::compile("resources.rc");
    }
}
