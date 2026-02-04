use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use zbus::interface;
use zvariant::{ObjectPath, Str, Value};

const MPRIS_BUS_NAME: &str = "org.mpris.MediaPlayer2.danavi";
const MPRIS_OBJECT_PATH: &str = "/org/mpris/MediaPlayer2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

impl PlaybackStatus {
    fn as_str(&self) -> &'static str {
        match self {
            PlaybackStatus::Playing => "Playing",
            PlaybackStatus::Paused => "Paused",
            PlaybackStatus::Stopped => "Stopped",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Song {
    #[allow(dead_code)]
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<i64>,
}

pub struct MprisState {
    pub playback_status: PlaybackStatus,
    pub current_song: Option<Song>,
    pub current_song_url: Option<String>,
    pub volume: f64,
}

impl Default for MprisState {
    fn default() -> Self {
        Self {
            playback_status: PlaybackStatus::Stopped,
            current_song: None,
            current_song_url: None,
            volume: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct PlayerInterface {
    state: Arc<RwLock<MprisState>>,
    command_sender: mpsc::UnboundedSender<MprisCommand>,
}

#[derive(Debug, Clone)]
pub enum MprisCommand {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    #[allow(dead_code)]
    Seek(i64),
    #[allow(dead_code)]
    SetPosition(i64),
    SetVolume(f64),
}

impl PlayerInterface {
    pub fn new(
        state: Arc<RwLock<MprisState>>,
        command_sender: mpsc::UnboundedSender<MprisCommand>,
    ) -> Self {
        Self {
            state,
            command_sender,
        }
    }

    async fn get_metadata_dict(&self) -> HashMap<String, Value<'static>> {
        let state = self.state.read().await;
        let mut metadata: HashMap<String, Value<'static>> = HashMap::new();

        metadata.insert(
            "mpris:trackid".to_string(),
            Value::ObjectPath(ObjectPath::from_str_unchecked("/org/mpris/MediaPlayer2/Track/0")),
        );

        if let Some(song) = &state.current_song {
            metadata.insert(
                "xesam:title".to_string(),
                Value::Str(Str::from(song.title.clone())),
            );
            
            if let Some(artist) = &song.artist {
                let artist_values: Vec<Value> = vec![Value::Str(Str::from(artist.clone()))];
                metadata.insert(
                    "xesam:artist".to_string(),
                    Value::Array(artist_values.into()),
                );
            }
            
            if let Some(album) = &song.album {
                metadata.insert(
                    "xesam:album".to_string(),
                    Value::Str(Str::from(album.clone())),
                );
            }
            
            if let Some(duration) = song.duration {
                metadata.insert(
                    "mpris:length".to_string(),
                    Value::I64(duration * 1_000_000),
                );
            }
            
            if let Some(url) = &state.current_song_url {
                metadata.insert(
                    "xesam:url".to_string(),
                    Value::Str(Str::from(url.clone())),
                );
            }
        }

        metadata
    }
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl PlayerInterface {
    #[zbus(name = "Next")]
    async fn next(&self) {
        let _ = self.command_sender.send(MprisCommand::Next);
    }

    #[zbus(name = "Previous")]
    async fn previous(&self) {
        let _ = self.command_sender.send(MprisCommand::Previous);
    }

    #[zbus(name = "Pause")]
    async fn pause(&self) {
        let _ = self.command_sender.send(MprisCommand::Pause);
    }

    #[zbus(name = "PlayPause")]
    async fn play_pause(&self) {
        let _ = self.command_sender.send(MprisCommand::PlayPause);
    }

    #[zbus(name = "Stop")]
    async fn stop(&self) {
        let _ = self.command_sender.send(MprisCommand::Stop);
    }

    #[zbus(name = "Play")]
    async fn play(&self) {
        let _ = self.command_sender.send(MprisCommand::Play);
    }

    #[zbus(name = "Seek")]
    async fn seek(&self, offset: i64) {
        let _ = self.command_sender.send(MprisCommand::Seek(offset));
    }

    #[zbus(name = "SetPosition")]
    async fn set_position(&self, _track_id: ObjectPath<'_>, position: i64) {
        let _ = self.command_sender.send(MprisCommand::SetPosition(position));
    }

    #[zbus(name = "OpenUri")]
    async fn open_uri(&self, _uri: &str) {
    }

    #[zbus(property, name = "PlaybackStatus")]
    async fn playback_status(&self) -> String {
        let state = self.state.read().await;
        state.playback_status.as_str().to_string()
    }

    #[zbus(property, name = "LoopStatus")]
    async fn loop_status(&self) -> String {
        "None".to_string()
    }

    #[zbus(property, name = "Rate")]
    async fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property, name = "Shuffle")]
    async fn shuffle(&self) -> bool {
        false
    }

    #[zbus(property, name = "Metadata")]
    async fn metadata(&self) -> HashMap<String, Value<'static>> {
        self.get_metadata_dict().await
    }

    #[zbus(property, name = "Volume")]
    async fn volume(&self) -> f64 {
        let state = self.state.read().await;
        state.volume
    }

    #[zbus(property, name = "Volume")]
    async fn set_volume(&self, volume: f64) {
        let _ = self.command_sender.send(MprisCommand::SetVolume(volume));
    }

    #[zbus(property, name = "Position")]
    async fn position(&self) -> i64 {
        0
    }

    #[zbus(property, name = "MinimumRate")]
    async fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property, name = "MaximumRate")]
    async fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property, name = "CanGoNext")]
    async fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property, name = "CanGoPrevious")]
    async fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(property, name = "CanPlay")]
    async fn can_play(&self) -> bool {
        true
    }

    #[zbus(property, name = "CanPause")]
    async fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property, name = "CanSeek")]
    async fn can_seek(&self) -> bool {
        false
    }

    #[zbus(property, name = "CanControl")]
    async fn can_control(&self) -> bool {
        true
    }
}

pub struct RootInterface;

#[interface(name = "org.mpris.MediaPlayer2")]
impl RootInterface {
    #[zbus(name = "Raise")]
    fn raise(&self) {
    }

    #[zbus(name = "Quit")]
    fn quit(&self) {
    }

    #[zbus(property, name = "CanQuit")]
    fn can_quit(&self) -> bool {
        false
    }

    #[zbus(property, name = "CanRaise")]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property, name = "HasTrackList")]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property, name = "Identity")]
    fn identity(&self) -> String {
        "danavi".to_string()
    }

    #[zbus(property, name = "DesktopEntry")]
    fn desktop_entry(&self) -> String {
        "danavi".to_string()
    }

    #[zbus(property, name = "SupportedUriSchemes")]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec![]
    }

    #[zbus(property, name = "SupportedMimeTypes")]
    fn supported_mime_types(&self) -> Vec<String> {
        vec![]
    }
}

pub struct MprisServer {
    #[allow(dead_code)]
    connection: zbus::Connection,
    #[allow(dead_code)]
    state: Arc<RwLock<MprisState>>,
}

impl MprisServer {
    pub async fn new(
        command_sender: mpsc::UnboundedSender<MprisCommand>,
    ) -> anyhow::Result<(Self, Arc<RwLock<MprisState>>)> {
        let state = Arc::new(RwLock::new(MprisState::default()));
        
        let player_interface = PlayerInterface::new(state.clone(), command_sender);
        let root_interface = RootInterface;

        let connection = zbus::connection::Builder::session()?
            .name(MPRIS_BUS_NAME)?
            .serve_at(MPRIS_OBJECT_PATH, player_interface)?
            .serve_at(MPRIS_OBJECT_PATH, root_interface)?
            .build()
            .await?;

        Ok((Self { connection, state: state.clone() }, state))
    }

    #[allow(dead_code)]
    pub async fn update_playback_status(&self, status: PlaybackStatus) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        state.playback_status = status;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn update_current_song(&self, song: Option<Song>, url: Option<String>) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        state.current_song = song;
        state.current_song_url = url;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn update_volume(&self, volume: f64) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        state.volume = volume;
        Ok(())
    }
}
