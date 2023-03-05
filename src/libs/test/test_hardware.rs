
// For use in manually-invoked tests, on the various systems I run these on. What does PortAudio
// need as the name of the actual loudspeaker device name? It varies between operating systems and
// hardware.
// If you're developing digimorse and can identify /your/ speaker, please adjust this...
pub fn get_current_system_speaker_name() -> String {
    #[cfg(target_os = "macos")]
    return macos_speaker();
    #[cfg(target_os = "windows")]
    return windows_speaker();
}

#[cfg(target_os = "macos")]
fn macos_speaker() -> String {
    use log::debug;

    let info = os_info::get();
    debug!("OS info is {:?}", info);
    match info.version() {
        os_info::Version::Semantic(x,y,_z) => {
            if *x == 10_u64 && *y == 13_u64 { // High Sierra
                return "Built-in Output".to_owned();
            }
            if *x == 10_u64 && *y == 15_u64 { // Catalina
                return "Built-in Output".to_owned();
            }
            if *x == 13_u64 { // Ventura
                return "MacBook Pro Speakers".to_owned();
            }
            panic!("test_hardware::macos_speaker() hasn't been modified to work on macOS {:?}", info);
        }
        _ => {
            panic!("test_hardware::macos_speaker() doesn't know what macOS this is!")
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_speaker() -> String {
    use log::debug;
    debug!("I'm a windows system, I may not have the right speaker defined");
    return "Speakers (Realtek High Definition Audio)".to_owned();
}