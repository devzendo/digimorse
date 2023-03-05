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
    let info = os_info::get();
    debug!("OS version is {}", info.version());
    //if info.version()
    let old_macbook = false; // TODO: determine the device at runtime
    let dev = if old_macbook {"Built-in Output"} else {"MacBook Pro Speakers"};
    return dev.to_owned();
}

#[cfg(target_os = "windows")]
fn windows_speaker() -> String {
   return "Speakers (Realtek High Definition Audio)".to_owned();
}