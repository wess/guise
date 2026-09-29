// `native` = the embedded wry web view is compiled in. It is the `webview`
// feature minus Linux: wry can only parent onto an Xlib window handle, and
// gpui's X11 backend never implements `HasWindowHandle` while its Wayland one
// hands over a Wayland handle wry rejects — so there is no working path there
// and `WebView` stays the themed placeholder instead of failing at runtime.
fn main() {
  println!("cargo:rustc-check-cfg=cfg(native)");
  println!("cargo:rerun-if-changed=build.rs");
  let webview = std::env::var_os("CARGO_FEATURE_WEBVIEW").is_some();
  let linux = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "linux");
  if webview && !linux {
    println!("cargo:rustc-cfg=native");
  }
}
