use std::path::PathBuf;

pub struct LocalStorage {
    base_dir: PathBuf,
}

impl LocalStorage {
    pub fn new(base_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&base_dir).expect("failed to create storage directory");
        Self { base_dir }
    }

    fn package_dir(&self, name: &str) -> PathBuf {
        self.base_dir.join(name.replace('/', "_"))
    }

    fn wasm_path(&self, name: &str, version: &str) -> PathBuf {
        self.package_dir(name).join(format!("{}.wasm", version))
    }

    pub fn store_wasm(&self, name: &str, version: &str, data: &[u8]) -> Result<(), String> {
        let dir = self.package_dir(name);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("failed to create package directory: {}", e))?;

        let path = self.wasm_path(name, version);
        std::fs::write(&path, data)
            .map_err(|e| format!("failed to write wasm binary: {}", e))?;

        Ok(())
    }

    pub fn read_wasm(&self, name: &str, version: &str) -> Option<Vec<u8>> {
        let path = self.wasm_path(name, version);
        std::fs::read(&path).ok()
    }

    #[allow(dead_code)]
    pub fn delete_wasm(&self, name: &str, version: &str) -> bool {
        let path = self.wasm_path(name, version);
        std::fs::remove_file(&path).is_ok()
    }

    #[allow(dead_code)]
    pub fn wasm_exists(&self, name: &str, version: &str) -> bool {
        self.wasm_path(name, version).exists()
    }
}
