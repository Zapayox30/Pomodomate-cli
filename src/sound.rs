use std::path::{Path, PathBuf};

/// Extensions we try when the user names a sound without one.
const EXTENSIONS: &[&str] = &["mp3", "ogg", "wav", "flac"];

/// Directory holding the user's ambient tracks.
///
/// Pomodomate ships no audio of its own: you drop your own files here, so
/// there is never a licensing question about what is playing.
pub fn sounds_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("pomodomate").join("sounds"))
}

/// Turn a config value into a playable file path.
///
/// Accepts an absolute or relative path as-is, or a bare name like `rain`,
/// which is looked up in the sounds directory with each known extension.
pub fn resolve(name: &str, sounds_dir: Option<&Path>) -> Option<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }

    // An explicit path wins, so users can point anywhere on disk.
    let direct = Path::new(name);
    if direct.is_file() {
        return Some(direct.to_path_buf());
    }

    let dir = sounds_dir?;

    // A name that already carries an extension.
    let candidate = dir.join(name);
    if candidate.is_file() {
        return Some(candidate);
    }

    EXTENSIONS
        .iter()
        .map(|ext| dir.join(format!("{name}.{ext}")))
        .find(|path| path.is_file())
}

/// Plays a looping ambient track for the duration of a work phase.
///
/// Without the `audio` feature this is a no-op that keeps the rest of the code
/// free of `#[cfg]` noise.
#[derive(Default)]
pub struct Ambient {
    #[cfg(feature = "audio")]
    playing: Option<AudioHandles>,
}

/// Keeps the device and player alive; dropping either stops playback.
#[cfg(feature = "audio")]
struct AudioHandles {
    // Held purely so the output device stays open.
    _device: rodio::MixerDeviceSink,
    player: rodio::Player,
}

impl Ambient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start looping `path`, replacing anything already playing.
    ///
    /// Failures are swallowed on purpose: a missing codec or a busy sound card
    /// should never interrupt a pomodoro.
    #[cfg(feature = "audio")]
    pub fn play(&mut self, path: &Path) {
        use rodio::Source;

        self.stop();

        let Ok(device) = rodio::DeviceSinkBuilder::open_default_sink() else {
            return;
        };
        let player = rodio::Player::connect_new(device.mixer());

        let Ok(file) = std::fs::File::open(path) else {
            return;
        };
        let Ok(decoder) = rodio::Decoder::try_from(file) else {
            return;
        };

        player.append(decoder.repeat_infinite());
        player.play();

        self.playing = Some(AudioHandles {
            _device: device,
            player,
        });
    }

    #[cfg(not(feature = "audio"))]
    pub fn play(&mut self, _path: &Path) {}

    /// Whether a track is currently loaded and playing.
    #[cfg(feature = "audio")]
    pub fn is_playing(&self) -> bool {
        self.playing.is_some()
    }

    #[cfg(not(feature = "audio"))]
    pub fn is_playing(&self) -> bool {
        false
    }

    /// Stop and release the audio device.
    #[cfg(feature = "audio")]
    pub fn stop(&mut self) {
        if let Some(handles) = self.playing.take() {
            handles.player.stop();
        }
    }

    #[cfg(not(feature = "audio"))]
    pub fn stop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir()
            .join("pomodomate-sound-test")
            .join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn empty_names_resolve_to_nothing() {
        let dir = temp_dir();
        assert!(resolve("", Some(&dir)).is_none());
        assert!(resolve("   ", Some(&dir)).is_none());
    }

    #[test]
    fn a_bare_name_finds_a_file_with_a_known_extension() {
        let dir = temp_dir();
        std::fs::write(dir.join("rain.mp3"), b"not really audio").unwrap();

        assert_eq!(resolve("rain", Some(&dir)), Some(dir.join("rain.mp3")));
    }

    #[test]
    fn a_name_with_its_extension_is_used_directly() {
        let dir = temp_dir();
        std::fs::write(dir.join("fire.ogg"), b"x").unwrap();

        assert_eq!(resolve("fire.ogg", Some(&dir)), Some(dir.join("fire.ogg")));
    }

    #[test]
    fn an_explicit_path_bypasses_the_sounds_directory() {
        let elsewhere = temp_dir();
        let track = elsewhere.join("custom.wav");
        std::fs::write(&track, b"x").unwrap();

        let sounds = temp_dir();
        assert_eq!(
            resolve(track.to_str().unwrap(), Some(&sounds)),
            Some(track),
            "an absolute path should be honored as-is"
        );
    }

    #[test]
    fn a_missing_sound_resolves_to_nothing() {
        let dir = temp_dir();
        assert!(resolve("nope", Some(&dir)).is_none());
    }

    #[test]
    fn resolution_without_a_sounds_directory_is_safe() {
        assert!(resolve("rain", None).is_none());
    }

    #[test]
    fn extension_search_order_is_stable() {
        let dir = temp_dir();
        // Both exist: mp3 comes first in EXTENSIONS, so it should win.
        std::fs::write(dir.join("dual.wav"), b"x").unwrap();
        std::fs::write(dir.join("dual.mp3"), b"x").unwrap();

        assert_eq!(resolve("dual", Some(&dir)), Some(dir.join("dual.mp3")));
    }

    #[test]
    fn stopping_a_player_that_never_started_is_harmless() {
        let mut ambient = Ambient::new();
        ambient.stop();
        ambient.stop();
    }
}
