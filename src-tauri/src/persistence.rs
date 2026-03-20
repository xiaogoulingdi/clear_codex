use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::Context;

use crate::models::AppSnapshot;

#[derive(Debug, Clone)]
pub struct PersistenceStore {
  path: PathBuf,
}

impl PersistenceStore {
  pub fn new(base_dir: &Path) -> anyhow::Result<Self> {
    fs::create_dir_all(base_dir).with_context(|| {
      format!("failed to create app data directory at {}", base_dir.display())
    })?;

    Ok(Self {
      path: base_dir.join("clear-codex-state.json"),
    })
  }

  pub fn load(&self) -> anyhow::Result<AppSnapshot> {
    if !self.path.exists() {
      return Ok(AppSnapshot::default());
    }

    let raw = fs::read_to_string(&self.path)
      .with_context(|| format!("failed to read {}", self.path.display()))?;
    let snapshot = serde_json::from_str::<AppSnapshot>(&raw)
      .with_context(|| format!("failed to parse {}", self.path.display()))?;
    Ok(snapshot)
  }

  pub fn save(&self, snapshot: &AppSnapshot) -> anyhow::Result<()> {
    let raw = serde_json::to_string_pretty(snapshot)?;
    fs::write(&self.path, raw)
      .with_context(|| format!("failed to write {}", self.path.display()))?;
    Ok(())
  }
}
