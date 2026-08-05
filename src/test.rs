#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::player::CharacterItems;
    use std::error::Error;
    use std::path::PathBuf;
    use std::str::FromStr;

    #[test]
    fn v11_player_test() -> Result<(), Box<dyn Error>> {
        let config = Config::new();

        if config.installation_dir().is_none() {
            panic!("The game installation dir needs to be configured.");
        }

        if config.save_dir().is_none() {
            panic!("The save dir needs to be configured.");
        }

        if let Some(save_dir) = config.save_dir()
            && !save_dir.exists()
        {
            panic!("The configured save directory does not exist: {:?}", save_dir);
        }

        let mut dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR")).unwrap();
        dir.push("test/v11_player.gdc");

        assert!(CharacterItems::read(dir).is_ok());
        Ok(())
    }
}
