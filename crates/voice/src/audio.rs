use crate::errors::{VoiceError, VoiceResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub input: bool,
    pub output: bool,
    pub default_input: bool,
    pub default_output: bool,
}

#[cfg(feature = "native-audio")]
pub fn list_audio_devices() -> VoiceResult<Vec<AudioDeviceInfo>> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let default_input = host.default_input_device().and_then(|device| device.name().ok());
    let default_output = host.default_output_device().and_then(|device| device.name().ok());
    let devices = host
        .devices()
        .map_err(|err| VoiceError::Audio(err.to_string()))?;

    let mut results = Vec::new();
    for device in devices {
        let name = device.name().unwrap_or_else(|_| "unknown".into());
        let input = device.default_input_config().is_ok()
            || device
                .supported_input_configs()
                .map(|mut configs| configs.next().is_some())
                .unwrap_or(false);
        let output = device.default_output_config().is_ok()
            || device
                .supported_output_configs()
                .map(|mut configs| configs.next().is_some())
                .unwrap_or(false);
        results.push(AudioDeviceInfo {
            default_input: default_input.as_deref() == Some(name.as_str()),
            default_output: default_output.as_deref() == Some(name.as_str()),
            name,
            input,
            output,
        });
    }

    results.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(results)
}

#[cfg(not(feature = "native-audio"))]
pub fn list_audio_devices() -> VoiceResult<Vec<AudioDeviceInfo>> {
    Err(VoiceError::Audio(
        "native audio support not enabled; rebuild with kaya-voice/native-audio".into(),
    ))
}