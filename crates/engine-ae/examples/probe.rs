fn main() {
    println!("locate: {:?}", dpl_engine_ae::locate());
    let caps = dpl_engine_ae::Caps::probe();
    println!("caps: {caps:?}");
    for p in [
        dpl_engine_ae::Profile::Gentle,
        dpl_engine_ae::Profile::Balanced,
        dpl_engine_ae::Profile::MaxThroughput,
    ] {
        println!(
            "{p:?}: {:?}",
            dpl_engine_ae::RenderTuning::for_caps(caps, p)
        );
    }
    println!("prefs_file: {:?}", dpl_engine_ae::prefs_file());
    if let Some(p) = dpl_engine_ae::prefs_file() {
        println!(
            "welcome_blocking: {}",
            dpl_engine_ae::welcome_screen_blocking(&p)
        );
    }
    println!("ae_running: {}", dpl_engine_ae::ae_running());
}
