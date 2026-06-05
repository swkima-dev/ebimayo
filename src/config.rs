pub fn load_anthropic_api_key() -> String {
    dotenvy::from_filename(".env.local").ok();

    std::env::var("ANTHROPIC_API_KEY").expect("Key should be found.")
}
