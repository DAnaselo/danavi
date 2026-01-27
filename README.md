# danavi

A terminal-based music client for Navidrome/Subsonic servers, written in Rust.

## Features

- Browse your music library by artist, album, and songs
- Search functionality
- Queue management with play, add, remove, and clear
- Vim and arrow key navigation

### Requirements

- git
- rust
- ffmpeg

### Install/Build Commands

```bash
git clone https://github.com/DAnaselo/danavi.git
cd danavi
cargo install --path .
```
# Now Export The .cargo/bin to $PATH
```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
```
or
```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
```

## Configuration

The app will create a config file at:
`~/.config/danavi/config.json`

Edit this file to add your Subsonic server details:
```json
{
  "base_url": "http://localhost:4533",
  "username": "your-username",
  "password": "your-password",
  "show_easter_eggs": true
}
```

## Controls

### Navigation
- **↑/↓** or **j/k** - Navigate up/down in list
- **→/l** or **Enter** - Select item (drill down into albums/songs or play)
- **←/h** - Go back to previous view

### Search
- **/** or **i** - Open search
- **Enter** - Execute search
- **Escape** - Cancel search
- **Backspace** - Delete last character

### Queue
- **a** - Add current song to queue
- **n** - Play next song in queue
- **r** - Remove first song from queue
- **c** - Clear queue
- **p** - Start/restart queue from beginning
- **Space** - Pause/resume playback

### General
- **?** - Show help menu
- **q** or **Escape** - Quit app

## Project Structure

```
src/
├── main.rs      - Main application entry point
├── client.rs     - Subsonic API client
├── config.rs     - Configuration management
├── types.rs      - Type definitions
└── tui.rs        - Terminal UI implementation
```
