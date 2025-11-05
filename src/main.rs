mod config;
mod generation;
mod symlink;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use config::Config;
use generation::GenerationManager;
use symlink::SymlinkManager;

#[derive(Parser)]
#[command(name = "imp")]
#[command(about = "A generation-based symlink manager for impermanence", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply a configuration and create a new generation
    Apply {
        /// Path to the configuration file
        #[arg(short, long, default_value = "imp.toml")]
        config: PathBuf,

        /// Skip validation before applying
        #[arg(short, long)]
        skip_validation: bool,
    },

    /// List all generations
    List,

    /// Show information about a specific generation
    Show {
        /// Generation number to show
        number: u64,
    },

    /// Switch to a different generation
    Switch {
        /// Generation number to switch to
        number: u64,
    },

    /// Delete a generation
    Delete {
        /// Generation number to delete
        number: u64,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Verify the current generation's symlinks
    Verify,

    /// Show the currently active generation
    Current,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Apply {
            config,
            skip_validation,
        } => apply_config(&config, skip_validation)?,
        Commands::List => list_generations()?,
        Commands::Show { number } => show_generation(number)?,
        Commands::Switch { number } => switch_generation(number)?,
        Commands::Delete { number, force } => delete_generation(number, force)?,
        Commands::Verify => verify_generation()?,
        Commands::Current => show_current_generation()?,
    }

    Ok(())
}

fn apply_config(config_path: &PathBuf, skip_validation: bool) -> Result<()> {
    println!("Loading configuration from: {}", config_path.display());

    let config = Config::from_file(config_path)?;

    if !skip_validation {
        println!("Validating configuration...");
        config.validate()?;
    }

    // Convert persistence config to symlinks
    let symlinks = config.to_symlinks();

    let symlink_manager = SymlinkManager::new();
    let generation_manager = GenerationManager::new(config.state_dir.clone())?;

    let next_gen = generation_manager.next_generation_number()?;
    println!("\nCreating generation {}...", next_gen);

    // Remove old symlinks if there's an active generation
    if let Some(active_gen) = generation_manager.get_active_generation()? {
        println!("Removing symlinks from generation {}...", active_gen.number);
        symlink_manager.remove(&active_gen.symlinks)?;
    }

    println!("\nApplying {} symlinks...", symlinks.len());
    let generation_symlinks = symlink_manager.apply(&symlinks)?;

    let generation =
        generation_manager.create_generation(config_path.clone(), generation_symlinks)?;

    println!(
        "\n✓ Successfully created and activated generation {}",
        generation.number
    );
    println!("  Created at: {}", generation.created_at);
    println!("  Symlinks: {}", generation.symlinks.len());

    Ok(())
}

fn list_generations() -> Result<()> {
    let state_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imp");

    let generation_manager = GenerationManager::new(state_dir)?;
    let generations = generation_manager.list_generations()?;

    if generations.is_empty() {
        println!("No generations found.");
        return Ok(());
    }

    println!("Generations:");
    for gen in generations {
        let active_marker = if gen.active { " (active)" } else { "" };
        println!(
            "  {} - {} - {} symlinks{}",
            gen.number,
            gen.created_at.format("%Y-%m-%d %H:%M:%S"),
            gen.symlinks.len(),
            active_marker
        );
    }

    Ok(())
}

fn show_generation(number: u64) -> Result<()> {
    let state_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imp");

    let generation_manager = GenerationManager::new(state_dir)?;
    let generations = generation_manager.list_generations()?;

    let gen = generations
        .iter()
        .find(|g| g.number == number)
        .ok_or_else(|| anyhow::anyhow!("Generation {} not found", number))?;

    println!("Generation {}:", gen.number);
    println!("  Created at: {}", gen.created_at);
    println!("  Active: {}", gen.active);
    println!("  Config: {}", gen.config_path.display());
    println!("  Symlinks:");

    for symlink in &gen.symlinks {
        println!(
            "    {} -> {}",
            symlink.target.display(),
            symlink.source.display()
        );
        if let Some(backup) = &symlink.backup_path {
            println!("      (backup: {})", backup.display());
        }
    }

    Ok(())
}

fn switch_generation(number: u64) -> Result<()> {
    let state_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imp");

    let generation_manager = GenerationManager::new(state_dir)?;
    let symlink_manager = SymlinkManager::new();

    // Remove current generation's symlinks
    if let Some(active_gen) = generation_manager.get_active_generation()? {
        println!("Removing symlinks from generation {}...", active_gen.number);
        symlink_manager.remove(&active_gen.symlinks)?;
    }

    // Switch to new generation
    let new_gen = generation_manager.switch_generation(number)?;

    println!("\nApplying symlinks from generation {}...", new_gen.number);

    // Recreate the symlinks
    for gen_symlink in &new_gen.symlinks {
        use std::os::unix::fs as unix_fs;

        if let Some(parent) = gen_symlink.target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        unix_fs::symlink(&gen_symlink.source, &gen_symlink.target)?;
        println!(
            "  ✓ Created symlink: {} -> {}",
            gen_symlink.target.display(),
            gen_symlink.source.display()
        );
    }

    println!("\n✓ Switched to generation {}", number);

    Ok(())
}

fn delete_generation(number: u64, force: bool) -> Result<()> {
    let state_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imp");

    let generation_manager = GenerationManager::new(state_dir)?;

    if !force {
        print!(
            "Are you sure you want to delete generation {}? (y/N): ",
            number
        );
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    generation_manager.delete_generation(number)?;
    println!("✓ Deleted generation {}", number);

    Ok(())
}

fn verify_generation() -> Result<()> {
    let state_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imp");

    let generation_manager = GenerationManager::new(state_dir)?;
    let symlink_manager = SymlinkManager::new();

    let active_gen = generation_manager
        .get_active_generation()?
        .ok_or_else(|| anyhow::anyhow!("No active generation"))?;

    println!("Verifying generation {}...", active_gen.number);

    let errors = symlink_manager.verify(&active_gen.symlinks)?;

    if errors.is_empty() {
        println!("✓ All symlinks are correctly configured");
    } else {
        println!("✗ Found {} error(s):", errors.len());
        for error in errors {
            println!("  - {}", error);
        }
    }

    Ok(())
}

fn show_current_generation() -> Result<()> {
    let state_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imp");

    let generation_manager = GenerationManager::new(state_dir)?;

    if let Some(gen) = generation_manager.get_active_generation()? {
        println!("Current generation: {}", gen.number);
        println!("  Created at: {}", gen.created_at);
        println!("  Config: {}", gen.config_path.display());
        println!("  Symlinks: {}", gen.symlinks.len());
    } else {
        println!("No active generation");
    }

    Ok(())
}
