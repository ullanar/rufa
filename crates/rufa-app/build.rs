fn main() {
    println!("cargo::rerun-if-changed=assets/icons/rufa.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("assets/icons/rufa.ico");
    resource.set("ProductName", "Rufa");
    resource.set("FileDescription", "EVE Online fitting tool");
    resource.set("CompanyName", "ullanar");
    resource.compile().expect("the windows resource compiler");
}
