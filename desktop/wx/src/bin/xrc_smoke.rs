use wxdragon::xrc::XmlResource;

const XRC_DATA: &str = include_str!("../../ui/main.xrc");

fn main() {
    wxdragon::main(|_| {
        let resource = XmlResource::get();
        resource.init_all_handlers();
        resource.init_platform_aware_staticbitmap_handler();
        resource.init_sizer_handlers();

        if let Err(err) = resource.load_from_string(XRC_DATA) {
            eprintln!("XRC load failed: {err}");
            std::process::exit(1);
        }

        if resource.load_frame(None, "main_frame").is_none() {
            eprintln!("XRC root object missing: main_frame");
            std::process::exit(1);
        }

        std::process::exit(0);
    })
    .expect("wxDragon XRC smoke test failed");
}
