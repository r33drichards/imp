use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Represents a single generation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Generation {
    /// Generation number (monotonically increasing)
    pub number: u64,

    /// When this generation was created
    pub created_at: DateTime<Utc>,

    /// Path to the config file used
    pub config_path: PathBuf,

    /// List of symlinks that were created
    pub symlinks: Vec<GenerationSymlink>,

    /// Whether this generation is currently active
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerationSymlink {
    pub source: PathBuf,
    pub target: PathBuf,
    /// If a backup was created, store its path
    pub backup_path: Option<PathBuf>,
}

pub struct GenerationManager {
    state_dir: PathBuf,
    generations_file: PathBuf,
}

impl GenerationManager {
    pub fn new(state_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&state_dir)?;
        let generations_file = state_dir.join("generations.json");

        Ok(Self {
            state_dir,
            generations_file,
        })
    }

    /// Load all generations from disk
    pub fn load_generations(&self) -> Result<Vec<Generation>> {
        if !self.generations_file.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&self.generations_file)?;
        let generations: Vec<Generation> = serde_json::from_str(&contents)?;
        Ok(generations)
    }

    /// Save generations to disk
    fn save_generations(&self, generations: &[Generation]) -> Result<()> {
        let contents = serde_json::to_string_pretty(generations)?;
        fs::write(&self.generations_file, contents)?;
        Ok(())
    }

    /// Get the next generation number
    pub fn next_generation_number(&self) -> Result<u64> {
        let generations = self.load_generations()?;
        Ok(generations.iter().map(|g| g.number).max().unwrap_or(0) + 1)
    }

    /// Create a new generation
    pub fn create_generation(
        &self,
        config_path: PathBuf,
        symlinks: Vec<GenerationSymlink>,
    ) -> Result<Generation> {
        let mut generations = self.load_generations()?;

        // Deactivate all previous generations
        for gen in &mut generations {
            gen.active = false;
        }

        let generation = Generation {
            number: self.next_generation_number()?,
            created_at: Utc::now(),
            config_path,
            symlinks,
            active: true,
        };

        generations.push(generation.clone());
        self.save_generations(&generations)?;

        Ok(generation)
    }

    /// Get the currently active generation
    pub fn get_active_generation(&self) -> Result<Option<Generation>> {
        let generations = self.load_generations()?;
        Ok(generations.into_iter().find(|g| g.active))
    }

    /// List all generations
    pub fn list_generations(&self) -> Result<Vec<Generation>> {
        self.load_generations()
    }

    /// Switch to a specific generation
    pub fn switch_generation(&self, number: u64) -> Result<Generation> {
        let mut generations = self.load_generations()?;

        // Find the index first
        let gen_index = generations
            .iter()
            .position(|g| g.number == number)
            .context("Generation not found")?;

        // Deactivate all
        for g in &mut generations {
            g.active = false;
        }

        // Activate the selected one
        generations[gen_index].active = true;
        let result = generations[gen_index].clone();

        self.save_generations(&generations)?;
        Ok(result)
    }

    /// Delete a generation
    pub fn delete_generation(&self, number: u64) -> Result<()> {
        let mut generations = self.load_generations()?;

        if let Some(gen) = generations.iter().find(|g| g.number == number) {
            if gen.active {
                anyhow::bail!("Cannot delete active generation");
            }
        }

        generations.retain(|g| g.number != number);
        self.save_generations(&generations)?;
        Ok(())
    }
}
