fn main() {
    // Embed environment variables at compile time
    println!("cargo:rerun-if-env-changed=SUPABASE_URL");
    println!("cargo:rerun-if-env-changed=SUPABASE_ANON_KEY");
    
    // Set compile-time environment variables
    if let Ok(url) = std::env::var("SUPABASE_URL") {
        println!("cargo:rustc-env=SUPABASE_URL={}", url);
    } else {
        println!("cargo:rustc-env=SUPABASE_URL=https://your-project.supabase.co");
    }
    
    if let Ok(key) = std::env::var("SUPABASE_ANON_KEY") {
        println!("cargo:rustc-env=SUPABASE_ANON_KEY={}", key);
    } else {
        println!("cargo:rustc-env=SUPABASE_ANON_KEY=your-anon-key");
    }
    
    tauri_build::build()
}
