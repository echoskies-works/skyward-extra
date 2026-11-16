// `option_env!("SWEX_PREVIEW_OUT")` is read once, when this crate is compiled.
// Cargo does not watch that variable on its own, so without this the shots
// would keep landing wherever the last build pointed them.
fn main() {
    println!("cargo:rerun-if-env-changed=SWEX_PREVIEW_OUT");
}
