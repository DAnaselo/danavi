use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::sync::Arc;

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    sink: Arc<Sink>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()
            .context("Failed to create audio output stream. Make sure PipeWire/WirePlumber is running and audio devices are available.")?;

        let sink = Sink::try_new(&stream_handle).context("Failed to create audio sink")?;

        sink.set_volume(1.0);

        Ok(Self {
            _stream: Some(_stream),
            sink: Arc::new(sink),
        })
    }

    pub fn play_bytes(&self, bytes: Vec<u8>) -> Result<()> {
        self.sink.stop();
        self.sink.clear();
        self.sink.set_volume(1.0);

        let cursor = Cursor::new(bytes);

        match Decoder::new(cursor) {
            Ok(source) => {
                self.sink.append(source);
                self.sink.play();
                Ok(())
            }
            Err(e) => {
                anyhow::bail!(
                    "Failed to decode audio: {}. Unsupported format or corrupted data.",
                    e
                );
            }
        }
    }

    pub fn stop(&self) {
        self.sink.stop();
        self.sink.clear();
    }

    pub fn toggle_pause(&self) {
        if self.sink.is_paused() {
            self.sink.play();
        } else if !self.sink.empty() {
            self.sink.pause();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        panic!("AudioPlayer::default() called - please use AudioPlayer::new() instead for proper error handling");
    }
}
