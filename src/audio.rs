use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Source, Sink};
use std::io::Cursor;
use std::sync::Mutex;

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    handle: OutputStreamHandle,
    sink: Mutex<Option<Sink>>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()
            .context("Failed to create audio output stream. Make sure PipeWire/WirePlumber is running and audio devices are available.")?;

        let sink = Sink::try_new(&stream_handle).context("Failed to create audio sink")?;
        sink.set_volume(1.0);

        Ok(Self {
            _stream: Some(_stream),
            handle: stream_handle,
            sink: Mutex::new(Some(sink)),
        })
    }

    pub fn play_bytes(&self, bytes: Vec<u8>) -> Result<()> {
        let cursor = Cursor::new(bytes);

        let source = Decoder::new(cursor).map_err(|e| {
            anyhow::anyhow!(
                "Failed to decode audio: {}. The server may have returned an unsupported format.",
                e
            )
        })?;

        if source.channels() == 0 || source.sample_rate() == 0 {
            anyhow::bail!(
                "Decoder produced invalid source (0 channels or 0 sample rate). \
                 The server may have returned empty or invalid audio data."
            );
        }

        let mut sink_guard = self.sink.lock().unwrap();

        // Create a fresh sink for this song to avoid race conditions
        // with stop/clear/append/play on a reused sink
        let new_sink = Sink::try_new(&self.handle)
            .context("Failed to create audio sink")?;
        new_sink.set_volume(1.0);
        new_sink.append(source);
        new_sink.play();

        *sink_guard = Some(new_sink);
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.stop();
            sink.clear();
        }
    }

    pub fn toggle_pause(&self) {
        let sink_guard = self.sink.lock().unwrap();
        if let Some(sink) = sink_guard.as_ref() {
            if sink.is_paused() {
                sink.play();
            } else if !sink.empty() {
                sink.pause();
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        self.sink
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.is_paused())
            .unwrap_or(false)
    }

    pub fn is_finished(&self) -> bool {
        self.sink
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.empty())
            .unwrap_or(true)
    }

    pub fn set_volume(&self, volume: f64) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.set_volume(volume as f32);
        }
    }

    pub fn get_volume(&self) -> f64 {
        self.sink
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.volume() as f64)
            .unwrap_or(1.0)
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        panic!("AudioPlayer::default() called - please use AudioPlayer::new() instead for proper error handling");
    }
}
