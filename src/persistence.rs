
pub(crate) mod media_info {
    use turbosql::serde_json;
    use crate::media_info::State;

    #[derive(Debug, Clone)]
    pub enum LoadError {
        File,
        Format,
    }

    #[derive(Debug, Clone)]
    pub enum SaveError {
        File,
        Write,
        Format,
    }
    #[cfg(not(target_arch = "wasm32"))]
    impl State {
        fn path() -> std::path::PathBuf {
            let mut path = if let Some(project_dirs) =
                directories_next::ProjectDirs::from("me", "zoarial", "media_manager")
            {
                project_dirs.data_dir().into()
            } else {
                std::env::current_dir().unwrap_or_default()
            };

            path.push("state.json");

            path
        }

        pub(crate) async fn load() -> Result<State, LoadError> {
            use async_std::prelude::*;

            let mut contents = String::new();

            let mut file = async_std::fs::File::open(Self::path())
                .await
                .map_err(|_| LoadError::File)?;

            file.read_to_string(&mut contents)
                .await
                .map_err(|_| LoadError::File)?;

            serde_json::from_str(&contents).map_err(|_| LoadError::Format)
        }

        pub(crate) async fn save(self) -> Result<(), SaveError> {
            use async_std::prelude::*;

            println!("Saving...");

            let json = serde_json::to_string_pretty(&self)
                .map_err(|_| SaveError::Format)?;

            let path = Self::path();

            if let Some(dir) = path.parent() {
                async_std::fs::create_dir_all(dir)
                    .await
                    .map_err(|_| SaveError::File)?;
            }

            {
                let mut file = async_std::fs::File::create(path)
                    .await
                    .map_err(|_| SaveError::File)?;

                file.write_all(json.as_bytes())
                    .await
                    .map_err(|_| SaveError::Write)?;
            }

            // This is a simple way to save at most once every couple seconds
            async_std::task::sleep(std::time::Duration::from_secs(2)).await;
            Ok(())
        }
    }

}