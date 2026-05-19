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
pub struct Album {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub duration: Option<i64>,
}

#[derive(Debug, Clone)]
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
}

// API Response types
#[derive(Debug, Deserialize)]
pub struct ArtistsResponse {
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
    pub artist: ArtistDetail,
}

#[derive(Debug, Deserialize)]
pub struct ArtistDetail {
    pub name: String,
    pub album: Vec<AlbumData>,
}

#[derive(Debug, Deserialize)]
pub struct AlbumData {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AlbumResponse {
    pub album: AlbumDetail,
}

#[derive(Debug, Deserialize)]
pub struct AlbumDetail {
    pub name: String,
    pub artist: Option<String>,
    pub song: Vec<SongData>,
}

#[derive(Debug, Deserialize)]
pub struct SongData {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    #[serde(default)]
    pub duration: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
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
    pub album: Option<String>,
    #[serde(default)]
    pub duration: Option<i64>,
}


