use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(alias = "baseUrl")]
    pub base_url: String,
    pub username: String,
    pub password: String,
    #[serde(default = "default_true", alias = "showEasterEggs")]
    pub show_easter_eggs: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone)]
pub struct Artist {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist_id: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub album_id: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<i64>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SearchResultItem {
    Album {
        id: String,
        name: String,
        artist: String,
        artist_id: String,
    },
    Song {
        id: String,
        title: String,
        artist: String,
        album_id: String,
        album: Option<String>,
        duration: Option<i64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewType {
    Artists,
    Albums,
    Songs,
    Search,
}

#[derive(Debug, Clone)]
pub enum PlaybackSource {
    Queue,
    Album {
        album_songs: Vec<Song>,
        current_index: usize,
    },
    Search,
    Other,
}

// API Response types
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SubsonicResponse<T> {
    #[serde(rename = "subsonic-response")]
    pub subsonic_response: T,
}

#[derive(Debug, Deserialize)]
pub struct ArtistsResponse {
    #[allow(dead_code)]
    pub status: String,
    pub artists: ArtistsData,
}

#[derive(Debug, Deserialize)]
pub struct ArtistsData {
    pub index: Vec<IndexEntry>,
}

#[derive(Debug, Deserialize)]
pub struct IndexEntry {
    pub artist: Vec<ArtistData>,
}

#[derive(Debug, Deserialize)]
pub struct ArtistData {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ArtistResponse {
    #[allow(dead_code)]
    pub status: String,
    pub artist: ArtistDetail,
}

#[derive(Debug, Deserialize)]
pub struct ArtistDetail {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub album: Vec<AlbumData>,
}

#[derive(Debug, Deserialize)]
pub struct AlbumData {
    pub id: String,
    pub name: String,
    pub artist_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AlbumResponse {
    #[allow(dead_code)]
    pub status: String,
    pub album: AlbumDetail,
}

#[derive(Debug, Deserialize)]
pub struct AlbumDetail {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub song: Vec<SongData>,
}

#[derive(Debug, Deserialize)]
pub struct SongData {
    pub id: String,
    pub title: String,
    pub album_id: Option<String>,
    pub artist: Option<String>,
    #[allow(dead_code)]
    pub album: Option<String>,
    #[serde(default)]
    pub duration: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    #[allow(dead_code)]
    pub status: String,
    #[serde(rename = "searchResult3")]
    pub search_result3: Option<SearchResult3>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult3 {
    pub album: Option<Vec<SearchAlbum>>,
    pub song: Option<Vec<SearchSong>>,
}

#[derive(Debug, Deserialize)]
pub struct SearchAlbum {
    pub id: String,
    pub name: String,
    pub artist: String,
    #[serde(rename = "artistId")]
    pub artist_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchSong {
    pub id: String,
    pub title: String,
    pub artist: String,
    #[serde(rename = "albumId")]
    pub album_id: String,
    pub album: Option<String>,
    #[serde(default)]
    pub duration: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ErrorResponse {
    pub status: String,
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ErrorDetail {
    pub code: Option<i32>,
    pub message: Option<String>,
}
