use std::fs;

fn main() {
    // Read package.json at build time
    let package_json = fs::read_to_string("package.json")
        .expect("Failed to read package.json");
    
    // Parse JSON to extract version
    let json: serde_json::Value = serde_json::from_str(&package_json)
        .expect("Failed to parse package.json");
    
    let version = json["version"]
        .as_str()
        .expect("Version field not found in package.json");
    
    // Set environment variable for use in the binary
    println!("cargo:rustc-env=PKG_VERSION={}", version);
    
    // Rerun build script if package.json changes
    println!("cargo:rerun-if-changed=package.json");
}
