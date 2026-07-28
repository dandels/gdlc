use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub struct Config {
    map: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        let mut config_path = match std::env::home_dir() {
            Some(p) => p,
            None => return Self { map }, // early return
        };
        #[cfg(target_os = "windows")]
        {
            config_path.push(".gdlc.conf");
        }
        #[cfg(not(target_os = "windows"))]
        {
            config_path.push(".config/");
            config_path.push("gdlc/");
            config_path.push("gdlc.conf");
        }
        if let Ok(mut file) = File::open(&config_path) {
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();
            for line in buf.lines() {
                if let Some((key, value)) = line.split_once('=')
                    && !key.is_empty()
                    && !value.is_empty()
                {
                    map.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }
        Self { map }
    }

    pub fn installation_dir(&self) -> Option<PathBuf> {
        self.map.get("installation_dir").map(PathBuf::from)
    }

    pub fn save_dir(&self) -> Option<PathBuf> {
        self.map.get("save_dir").map(PathBuf::from)
    }

    pub fn get_save_files(&self) -> Vec<PathBuf> {
        let mut ret = Vec::new();
        if self.save_dir().is_none() {
            return ret;
        }
        let save_dir = self.save_dir().unwrap();
        if let Ok(read_dir) = std::fs::read_dir(save_dir.join("main")) {
            read_dir.for_each(|d| {
                if let Ok(d) = d {
                    let gdc = d.path().join("player.gdc");
                    if gdc.exists() {
                        ret.push(gdc);
                    }
                }
            });
        }
        ret
    }

    pub fn get_stash_files(&self) -> (Option<PathBuf>, Option<PathBuf>) {
        if self.save_dir().is_none() {
            return (None, None);
        }
        let save_dir = self.save_dir().unwrap();
        let softcore_stash = save_dir.join("transfer.gst");
        let hardcore_stash = save_dir.join("transfer.gsh");

        (softcore_stash.exists().then_some(softcore_stash), hardcore_stash.exists().then_some(hardcore_stash))
    }

    pub fn get_databases(&self) -> Vec<PathBuf> {
        if self.installation_dir().is_none() {
            return Vec::new();
        }
        let install_dir = self.installation_dir().unwrap();
        let mut paths = [
            install_dir.clone().join("database/database.arz"),  // base game
            install_dir.clone().join("gdx1/database/GDX1.arz"), // Ashes of Malmouth
            install_dir.clone().join("gdx2/database/GDX2.arz"), // Forgotten Gods
            install_dir.clone().join("gdx3/database/GDX3.arz"), // Fangs of Asterkarn
        ];
        return_valid_paths(&mut paths)
    }

    pub fn get_localization_files(&self) -> Vec<PathBuf> {
        let lang = "EN";
        if self.installation_dir().is_none() {
            return Vec::new();
        }
        let install_dir = self.installation_dir().unwrap();
        let mut paths = [
            install_dir.clone().join(format!("resources/Text_{lang}.arc")),
            install_dir.clone().join(format!("gdx1/resources/Text_{lang}.arc")),
            install_dir.clone().join(format!("gdx2/resources/Text_{lang}.arc")),
            install_dir.clone().join(format!("gdx3/resources/Text_{lang}.arc")),
        ];
        return_valid_paths(&mut paths)
    }
}

fn return_valid_paths(paths: &mut [PathBuf]) -> Vec<PathBuf> {
    let mut ret = Vec::new();
    for path in paths {
        if path.exists() {
            ret.push(path.to_path_buf());
        } else {
            let lowercase_name = path.file_name().unwrap().to_ascii_lowercase();
            path.set_file_name(lowercase_name);
            if path.exists() {
                ret.push(path.to_path_buf());
            }
        }
    }
    ret
}
