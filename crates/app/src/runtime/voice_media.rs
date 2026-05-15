use kaya_voice::VoiceConfig;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

#[cfg(target_os = "linux")]
use std::process::Stdio;
#[cfg(target_os = "linux")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(target_os = "linux")]
use tokio::process::Command;

#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(target_os = "windows")]
use cpal::{SampleFormat, SampleRate, Stream, StreamConfig, SupportedStreamConfigRange};
#[cfg(target_os = "windows")]
use std::collections::VecDeque;
#[cfg(target_os = "windows")]
use std::sync::{Arc, Mutex};

const VOICE_SAMPLE_RATE_HZ: u32 = 8_000;
const VOICE_CHANNELS: u16 = 1;
const PCM_BYTES_PER_SAMPLE: usize = 2;

#[derive(Debug)]
pub(super) enum VoiceRuntimeEvent {
    CapturedPcm { samples: Vec<i16> },
    BackendError { scope: &'static str, message: String },
}

pub(super) struct VoiceMediaRuntime {
    pub event_rx: mpsc::UnboundedReceiver<VoiceRuntimeEvent>,
    playback_tx: mpsc::UnboundedSender<Vec<i16>>,
    shutdown_tx: watch::Sender<bool>,
    capture_task: JoinHandle<()>,
    playback_task: JoinHandle<()>,
}

impl VoiceMediaRuntime {
    pub fn start(config: &VoiceConfig) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (playback_tx, playback_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let capture_task = spawn_capture_task(config.clone(), event_tx.clone(), shutdown_rx.clone());
        let playback_task = spawn_playback_task(config.clone(), playback_rx, event_tx, shutdown_rx);

        Self {
            event_rx,
            playback_tx,
            shutdown_tx,
            capture_task,
            playback_task,
        }
    }

    pub fn queue_playback(&self, samples: Vec<i16>) {
        let _ = self.playback_tx.send(samples);
    }

    pub fn stop(self) {
        let _ = self.shutdown_tx.send(true);
        self.capture_task.abort();
        self.playback_task.abort();
    }
}

#[cfg(target_os = "linux")]
fn spawn_capture_task(
    config: VoiceConfig,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(capture_loop(config, event_tx, shutdown_rx))
}

#[cfg(target_os = "linux")]
fn spawn_playback_task(
    config: VoiceConfig,
    playback_rx: mpsc::UnboundedReceiver<Vec<i16>>,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(playback_loop(config, playback_rx, event_tx, shutdown_rx))
}

#[cfg(target_os = "windows")]
fn spawn_capture_task(
    config: VoiceConfig,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || windows_capture_loop(config, event_tx, shutdown_rx))
}

#[cfg(target_os = "windows")]
fn spawn_playback_task(
    config: VoiceConfig,
    playback_rx: mpsc::UnboundedReceiver<Vec<i16>>,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || windows_playback_loop(
        config,
        playback_rx,
        event_tx,
        shutdown_rx,
    ))
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn spawn_capture_task(
    _config: VoiceConfig,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    _shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
            scope: "voice.capture",
            message: "voice capture backend not implemented for this platform".into(),
        });
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn spawn_playback_task(
    _config: VoiceConfig,
    _playback_rx: mpsc::UnboundedReceiver<Vec<i16>>,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    _shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
            scope: "voice.playback",
            message: "voice playback backend not implemented for this platform".into(),
        });
    })
}

#[cfg(target_os = "linux")]
async fn capture_loop(
    config: VoiceConfig,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let frame_bytes = pcm_frame_bytes(config.opus_frame_ms);
    let mut command = Command::new("arecord");
    command
        .arg("-q")
        .arg("-t")
        .arg("raw")
        .arg("-f")
        .arg("S16_LE")
        .arg("-r")
        .arg(VOICE_SAMPLE_RATE_HZ.to_string())
        .arg("-c")
        .arg(VOICE_CHANNELS.to_string())
        .arg("-D")
        .arg(audio_device_arg(&config.input_device))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                scope: "voice.capture",
                message: format!("failed to start arecord: {err}"),
            });
            return;
        }
    };

    let Some(mut stdout) = child.stdout.take() else {
        let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
            scope: "voice.capture",
            message: "arecord stdout unavailable".into(),
        });
        let _ = child.kill().await;
        return;
    };

    loop {
        let mut buffer = vec![0_u8; frame_bytes];
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    let _ = child.kill().await;
                    break;
                }
            }
            read = stdout.read_exact(&mut buffer) => {
                match read {
                    Ok(_) => {
                        let samples = buffer
                            .chunks_exact(2)
                            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                            .collect();
                        let _ = event_tx.send(VoiceRuntimeEvent::CapturedPcm { samples });
                    }
                    Err(err) => {
                        let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                            scope: "voice.capture",
                            message: format!("capture stream closed: {err}"),
                        });
                        let _ = child.kill().await;
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
async fn playback_loop(
    config: VoiceConfig,
    mut playback_rx: mpsc::UnboundedReceiver<Vec<i16>>,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut command = Command::new("aplay");
    command
        .arg("-q")
        .arg("-t")
        .arg("raw")
        .arg("-f")
        .arg("S16_LE")
        .arg("-r")
        .arg(VOICE_SAMPLE_RATE_HZ.to_string())
        .arg("-c")
        .arg(VOICE_CHANNELS.to_string())
        .arg("-D")
        .arg(audio_device_arg(&config.output_device))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                scope: "voice.playback",
                message: format!("failed to start aplay: {err}"),
            });
            return;
        }
    };

    let Some(mut stdin) = child.stdin.take() else {
        let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
            scope: "voice.playback",
            message: "aplay stdin unavailable".into(),
        });
        let _ = child.kill().await;
        return;
    };

    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    let _ = stdin.shutdown().await;
                    let _ = child.kill().await;
                    break;
                }
            }
            maybe_samples = playback_rx.recv() => {
                let Some(samples) = maybe_samples else {
                    let _ = stdin.shutdown().await;
                    let _ = child.kill().await;
                    break;
                };
                let mut bytes = Vec::with_capacity(samples.len() * PCM_BYTES_PER_SAMPLE);
                for sample in samples {
                    bytes.extend_from_slice(&sample.to_le_bytes());
                }
                if let Err(err) = stdin.write_all(&bytes).await {
                    let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                        scope: "voice.playback",
                        message: format!("playback write failed: {err}"),
                    });
                    let _ = child.kill().await;
                    break;
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_capture_loop(
    config: VoiceConfig,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shutdown_rx: watch::Receiver<bool>,
) {
    let host = cpal::default_host();
    let device = match resolve_input_device(&host, &config.input_device) {
        Ok(device) => device,
        Err(message) => {
            let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                scope: "voice.capture",
                message,
            });
            return;
        }
    };
    let supported = match select_supported_input_config(&device) {
        Ok(config) => config,
        Err(message) => {
            let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                scope: "voice.capture",
                message,
            });
            return;
        }
    };
    let stream_config = supported.with_sample_rate(SampleRate(VOICE_SAMPLE_RATE_HZ)).config();
    let frame_samples = pcm_frame_bytes(config.opus_frame_ms) / PCM_BYTES_PER_SAMPLE;
    let shared = Arc::new(Mutex::new(Vec::<i16>::new()));
    let data_tx = event_tx.clone();
    let error_tx = event_tx.clone();
    let stream = match build_input_stream(
        &device,
        &stream_config,
        supported.sample_format(),
        frame_samples,
        data_tx,
        shared,
        error_tx,
    ) {
        Ok(stream) => stream,
        Err(message) => {
            let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                scope: "voice.capture",
                message,
            });
            return;
        }
    };

    if let Err(err) = stream.play() {
        let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
            scope: "voice.capture",
            message: format!("failed to start input stream: {err}"),
        });
        return;
    }

    while !*shutdown_rx.borrow() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(stream);
}

#[cfg(target_os = "windows")]
fn windows_playback_loop(
    config: VoiceConfig,
    mut playback_rx: mpsc::UnboundedReceiver<Vec<i16>>,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shutdown_rx: watch::Receiver<bool>,
) {
    let host = cpal::default_host();
    let device = match resolve_output_device(&host, &config.output_device) {
        Ok(device) => device,
        Err(message) => {
            let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                scope: "voice.playback",
                message,
            });
            return;
        }
    };
    let supported = match select_supported_output_config(&device) {
        Ok(config) => config,
        Err(message) => {
            let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                scope: "voice.playback",
                message,
            });
            return;
        }
    };
    let stream_config = supported.with_sample_rate(SampleRate(VOICE_SAMPLE_RATE_HZ)).config();
    let queue = Arc::new(Mutex::new(VecDeque::<i16>::new()));
    let error_tx = event_tx.clone();
    let stream = match build_output_stream(
        &device,
        &stream_config,
        supported.sample_format(),
        queue.clone(),
        error_tx,
    ) {
        Ok(stream) => stream,
        Err(message) => {
            let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
                scope: "voice.playback",
                message,
            });
            return;
        }
    };

    if let Err(err) = stream.play() {
        let _ = event_tx.send(VoiceRuntimeEvent::BackendError {
            scope: "voice.playback",
            message: format!("failed to start output stream: {err}"),
        });
        return;
    }

    while !*shutdown_rx.borrow() {
        match playback_rx.try_recv() {
            Ok(samples) => {
                if let Ok(mut pending) = queue.lock() {
                    pending.extend(samples);
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
        }
    }

    drop(stream);
}

fn pcm_frame_bytes(frame_ms: u16) -> usize {
    ((VOICE_SAMPLE_RATE_HZ as usize * frame_ms as usize) / 1_000)
        * VOICE_CHANNELS as usize
        * PCM_BYTES_PER_SAMPLE
}

#[cfg(target_os = "linux")]
fn audio_device_arg(device: &str) -> &str {
    if device.trim().is_empty() || device.eq_ignore_ascii_case("default") {
        "default"
    } else {
        device
    }
}

#[cfg(target_os = "windows")]
fn resolve_input_device(host: &cpal::Host, configured: &str) -> Result<cpal::Device, String> {
    if configured.trim().is_empty() || configured.eq_ignore_ascii_case("default") {
        return host
            .default_input_device()
            .ok_or_else(|| "no default input device available".into());
    }

    let devices = host.devices().map_err(|err| err.to_string())?;
    devices
        .filter_map(|device| device.name().ok().map(|name| (device, name)))
        .find(|(_, name)| name.eq_ignore_ascii_case(configured))
        .map(|(device, _)| device)
        .ok_or_else(|| format!("input device not found: {configured}"))
}

#[cfg(target_os = "windows")]
fn resolve_output_device(host: &cpal::Host, configured: &str) -> Result<cpal::Device, String> {
    if configured.trim().is_empty() || configured.eq_ignore_ascii_case("default") {
        return host
            .default_output_device()
            .ok_or_else(|| "no default output device available".into());
    }

    let devices = host.devices().map_err(|err| err.to_string())?;
    devices
        .filter_map(|device| device.name().ok().map(|name| (device, name)))
        .find(|(_, name)| name.eq_ignore_ascii_case(configured))
        .map(|(device, _)| device)
        .ok_or_else(|| format!("output device not found: {configured}"))
}

#[cfg(target_os = "windows")]
fn select_supported_input_config(device: &cpal::Device) -> Result<SupportedStreamConfigRange, String> {
    device
        .supported_input_configs()
        .map_err(|err| err.to_string())?
        .find(|config| supports_voice_format(config))
        .ok_or_else(|| "no 8kHz mono input format available".into())
}

#[cfg(target_os = "windows")]
fn select_supported_output_config(device: &cpal::Device) -> Result<SupportedStreamConfigRange, String> {
    device
        .supported_output_configs()
        .map_err(|err| err.to_string())?
        .find(|config| supports_voice_format(config))
        .ok_or_else(|| "no 8kHz mono output format available".into())
}

#[cfg(target_os = "windows")]
fn supports_voice_format(config: &SupportedStreamConfigRange) -> bool {
    config.channels() == VOICE_CHANNELS
        && config.min_sample_rate().0 <= VOICE_SAMPLE_RATE_HZ
        && config.max_sample_rate().0 >= VOICE_SAMPLE_RATE_HZ
}

#[cfg(target_os = "windows")]
fn build_input_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    frame_samples: usize,
    event_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shared: Arc<Mutex<Vec<i16>>>,
    error_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
) -> Result<Stream, String> {
    let err_fn = move |err| {
        let _ = error_tx.send(VoiceRuntimeEvent::BackendError {
            scope: "voice.capture",
            message: format!("input stream error: {err}"),
        });
    };

    match sample_format {
        SampleFormat::I16 => device
            .build_input_stream(
                config,
                move |data: &[i16], _| push_input_frame_i16(data, frame_samples, &event_tx, &shared),
                err_fn,
                None,
            )
            .map_err(|err| err.to_string()),
        SampleFormat::U16 => device
            .build_input_stream(
                config,
                move |data: &[u16], _| push_input_frame_u16(data, frame_samples, &event_tx, &shared),
                err_fn,
                None,
            )
            .map_err(|err| err.to_string()),
        SampleFormat::F32 => device
            .build_input_stream(
                config,
                move |data: &[f32], _| push_input_frame_f32(data, frame_samples, &event_tx, &shared),
                err_fn,
                None,
            )
            .map_err(|err| err.to_string()),
        other => Err(format!("unsupported input sample format: {other:?}")),
    }
}

#[cfg(target_os = "windows")]
fn push_input_frame_i16(
    data: &[i16],
    frame_samples: usize,
    event_tx: &mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shared: &Arc<Mutex<Vec<i16>>>,
) {
    if let Ok(mut buffer) = shared.lock() {
        buffer.extend_from_slice(data);
        flush_input_buffer(&mut buffer, frame_samples, event_tx);
    }
}

#[cfg(target_os = "windows")]
fn push_input_frame_u16(
    data: &[u16],
    frame_samples: usize,
    event_tx: &mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shared: &Arc<Mutex<Vec<i16>>>,
) {
    if let Ok(mut buffer) = shared.lock() {
        buffer.extend(data.iter().copied().map(u16_to_i16));
        flush_input_buffer(&mut buffer, frame_samples, event_tx);
    }
}

#[cfg(target_os = "windows")]
fn push_input_frame_f32(
    data: &[f32],
    frame_samples: usize,
    event_tx: &mpsc::UnboundedSender<VoiceRuntimeEvent>,
    shared: &Arc<Mutex<Vec<i16>>>,
) {
    if let Ok(mut buffer) = shared.lock() {
        buffer.extend(data.iter().copied().map(f32_to_i16));
        flush_input_buffer(&mut buffer, frame_samples, event_tx);
    }
}

#[cfg(target_os = "windows")]
fn flush_input_buffer(
    buffer: &mut Vec<i16>,
    frame_samples: usize,
    event_tx: &mpsc::UnboundedSender<VoiceRuntimeEvent>,
) {
    while buffer.len() >= frame_samples {
        let frame = buffer.drain(..frame_samples).collect::<Vec<_>>();
        let _ = event_tx.send(VoiceRuntimeEvent::CapturedPcm { samples: frame });
    }
}

#[cfg(target_os = "windows")]
fn build_output_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    queue: Arc<Mutex<VecDeque<i16>>>,
    error_tx: mpsc::UnboundedSender<VoiceRuntimeEvent>,
) -> Result<Stream, String> {
    let err_fn = move |err| {
        let _ = error_tx.send(VoiceRuntimeEvent::BackendError {
            scope: "voice.playback",
            message: format!("output stream error: {err}"),
        });
    };

    match sample_format {
        SampleFormat::I16 => device
            .build_output_stream(
                config,
                move |data: &mut [i16], _| fill_output_i16(data, &queue),
                err_fn,
                None,
            )
            .map_err(|err| err.to_string()),
        SampleFormat::U16 => device
            .build_output_stream(
                config,
                move |data: &mut [u16], _| fill_output_u16(data, &queue),
                err_fn,
                None,
            )
            .map_err(|err| err.to_string()),
        SampleFormat::F32 => device
            .build_output_stream(
                config,
                move |data: &mut [f32], _| fill_output_f32(data, &queue),
                err_fn,
                None,
            )
            .map_err(|err| err.to_string()),
        other => Err(format!("unsupported output sample format: {other:?}")),
    }
}

#[cfg(target_os = "windows")]
fn fill_output_i16(data: &mut [i16], queue: &Arc<Mutex<VecDeque<i16>>>) {
    if let Ok(mut pending) = queue.lock() {
        for slot in data.iter_mut() {
            *slot = pending.pop_front().unwrap_or(0);
        }
    }
}

#[cfg(target_os = "windows")]
fn fill_output_u16(data: &mut [u16], queue: &Arc<Mutex<VecDeque<i16>>>) {
    if let Ok(mut pending) = queue.lock() {
        for slot in data.iter_mut() {
            *slot = i16_to_u16(pending.pop_front().unwrap_or(0));
        }
    }
}

#[cfg(target_os = "windows")]
fn fill_output_f32(data: &mut [f32], queue: &Arc<Mutex<VecDeque<i16>>>) {
    if let Ok(mut pending) = queue.lock() {
        for slot in data.iter_mut() {
            *slot = i16_to_f32(pending.pop_front().unwrap_or(0));
        }
    }
}

#[cfg(target_os = "windows")]
fn u16_to_i16(sample: u16) -> i16 {
    (sample as i32 - 32_768) as i16
}

#[cfg(target_os = "windows")]
fn f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

#[cfg(target_os = "windows")]
fn i16_to_u16(sample: i16) -> u16 {
    (sample as i32 + 32_768).clamp(0, u16::MAX as i32) as u16
}

#[cfg(target_os = "windows")]
fn i16_to_f32(sample: i16) -> f32 {
    sample as f32 / i16::MAX as f32
}

pub(super) fn pcm_has_voice_activity(samples: &[i16]) -> bool {
    if samples.is_empty() {
        return false;
    }
    let peak = samples.iter().map(|sample| sample.unsigned_abs()).max().unwrap_or(0);
    peak >= 1_200
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_voice_activity_from_peak_amplitude() {
        assert!(!pcm_has_voice_activity(&[0, 100, 300, -500]));
        assert!(pcm_has_voice_activity(&[0, 100, 300, -2_000]));
    }

    #[test]
    fn computes_pcm_frame_bytes_for_20ms() {
        assert_eq!(pcm_frame_bytes(20), 320);
    }
}