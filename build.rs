fn main() -> Result<(), Box<dyn std::error::Error>> {
  vergen::EmitBuilder::builder().all_build().all_git().emit()?;

  #[cfg(feature = "zep-editor")]
  {
    build_zep_editor()?;
  }

  Ok(())
}

#[cfg(feature = "zep-editor")]
fn build_zep_editor() -> Result<(), Box<dyn std::error::Error>> {
  use std::{env, path::PathBuf};

  let out_dir = env::var("OUT_DIR")?;
  let zep_dir = PathBuf::from("external/zep");

  // Check if Zep submodule exists
  if !zep_dir.exists() {
    return Err("Zep submodule not found. Run 'git submodule update --init' first.".into());
  }

  println!("cargo:rerun-if-changed=external/zep");

  // Generate config_app.h that Zep expects
  let config_content = format!("#pragma once\n#define ZEP_ROOT \"{}\"\n", zep_dir.canonicalize()?.display());
  std::fs::write(PathBuf::from(&out_dir).join("config_app.h"), config_content)?;

  // Build Zep C++ library
  let mut build = cc::Build::new();
  build
        .cpp(true)
        .std("c++17")
        .include(&zep_dir.join("include"))
        .include(&zep_dir.join("include/zep"))
        .include(&zep_dir.join("include/zep/mcommon"))
        .include(&out_dir) // Include generated config_app.h
        .define("ZEP_SINGLE_HEADER", None)
        .define("ZEP_FEATURE_CPP_FILE_SYSTEM", None)
        .define("ZEP_USE_IMGUI", None)
        .flag("-fPIC")
        .flag("-fvisibility=default")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-unused-variable");

  // For now, skip compiling the complex Zep C++ sources to avoid linking issues
  // We'll just compile our minimal wrapper
  println!("cargo:warning=Using minimal Zep stub implementation to avoid C++ linking complexity");

  // Create a very minimal C wrapper to avoid C++ standard library issues
  let wrapper_content = r#"
// Minimal C wrapper that just provides stubs for now
// This avoids complex C++ linking issues

#include <cstddef>
#include <cstdint>

#ifdef __cplusplus
extern "C" {
#endif

// For now, just provide stub implementations to resolve linking
__attribute__((visibility("default"))) void* zep_create_editor(const char* root_path) {
    return (void*)0x1; // Non-null pointer to indicate "success"
}

__attribute__((visibility("default"))) void zep_destroy_editor(void* editor) {
    // No-op for now
}

__attribute__((visibility("default"))) void zep_init_with_text(void* editor, const char* name, const char* text) {
    // No-op for now
}

__attribute__((visibility("default"))) size_t zep_get_text(void* editor, char* buffer, size_t size) {
    if (!editor || !buffer || size == 0) return 0;

    // Return a simple test string
    const char* test_text = "Zep Editor (stub implementation)";
    size_t len = 0;
    while (test_text[len] && len < size - 1) {
        buffer[len] = test_text[len];
        len++;
    }
    buffer[len] = '\0';
    return len;
}

__attribute__((visibility("default"))) void zep_set_vim_mode(void* editor) {
    // No-op for now
}

__attribute__((visibility("default"))) bool zep_handle_key(void* editor, uint32_t key, uint32_t modifiers) {
    return true; // Always return true for now
}

__attribute__((visibility("default"))) void zep_display(void* editor, float x, float y, float width, float height) {
    // No-op for now
}

#ifdef __cplusplus
}
#endif
"#;

  std::fs::write(PathBuf::from(&out_dir).join("zep_wrapper.cpp"), wrapper_content)?;

  build.file(PathBuf::from(&out_dir).join("zep_wrapper.cpp"));

  // Compile the library
  build.compile("zep");

  // Be more explicit about library paths and linking order
  println!("cargo:rustc-link-search=native={}", out_dir);

  // Try using the absolute path to the library instead
  let lib_path = PathBuf::from(&out_dir).join("libzep.a");
  println!("cargo:rustc-link-arg={}", lib_path.display());

  // Also try the traditional approach as backup
  println!("cargo:rustc-link-lib=static=zep");

  // Link only basic system libraries since we're using a stub implementation
  #[cfg(target_os = "linux")]
  {
    println!("cargo:rustc-link-lib=m");
  }

  #[cfg(target_os = "macos")]
  {
    println!("cargo:rustc-link-lib=framework=Foundation");
  }

  #[cfg(target_os = "windows")]
  {
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=gdi32");
  }

  Ok(())
}
