#[test]
fn should_expose_a_library_crate() {
    let _ = robust_coding_example::AppConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        database_url: String::new(),
    };
}
