mod config;
mod mkv;
mod subsource;

use clap::{Parser, Subcommand};
use colored::Colorize;
use config::ConfigManager;

#[derive(Parser)]
#[command(name = "sub")]
#[command(about = "CLI tool for subtitle management")]
#[command(version = "2.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List subtitle tracks in a video
    List { video: String },
    /// Extract subtitle from video
    Extract {
        video: String,
        #[arg(short, help = "Track ID")]
        track_id: usize,
        #[arg(short, help = "Output filename")]
        output: Option<String>,
    },
    /// Inject subtitle into video
    Inject {
        video: String,
        subtitle: String,
        #[arg(short, help = "Language code (default: ind)")]
        lang: Option<String>,
        #[arg(short, help = "Track name")]
        name: Option<String>,
    },
    /// Search & download subtitle from SubSource
    Search {
        query: String,
        #[arg(short, help = "Language (overrides default)")]
        lang: Option<String>,
        #[arg(short, help = "Release year")]
        year: Option<u32>,
        #[arg(short, help = "Output directory")]
        output_dir: Option<String>,
        #[arg(short = 'n', long, help = "Non-interactive")]
        non_interactive: bool,
        #[arg(short, long, help = "Verbose")]
        verbose: bool,
    },
    /// Download subtitle by ID
    Download {
        #[arg(help = "Movie ID")]
        movie_id: u64,
        #[arg(help = "Subtitle ID")]
        sub_id: u64,
        #[arg(short, help = "Output directory")]
        output_dir: Option<String>,
    },
    /// API key management
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
    /// Config management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum KeyAction {
    /// Configure API key
    Setup,
    /// Add/update API key
    Add { key: Option<String> },
    /// Show masked API key
    Show,
    /// Remove API key
    Remove,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set config value
    Set { key: String, value: String },
    /// Show current config
    Show,
    /// Reset config
    Reset,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let cfg = ConfigManager::load();

    match cli.command {
        Commands::List { video } => {
            mkv::list_subtitles(&video, &cfg.default_lang);
        }
        Commands::Extract {
            video,
            track_id,
            output,
        } => {
            mkv::extract_subtitle(&video, track_id, output.as_deref());
        }
        Commands::Inject {
            video,
            subtitle,
            lang,
            name,
        } => {
            mkv::inject_subtitle(&video, &subtitle, lang.as_deref(), name.as_deref());
        }
        Commands::Search {
            query,
            lang,
            year,
            output_dir,
            non_interactive,
            verbose,
        } => {
            let language = lang.unwrap_or_else(|| cfg.default_lang.clone());
            let out_dir = output_dir.unwrap_or_else(|| cfg.default_dir.clone());
            subsource::search(
                &cfg,
                &query,
                &language,
                year,
                &out_dir,
                non_interactive,
                verbose,
            )
            .await;
        }
        Commands::Download {
            movie_id,
            sub_id,
            output_dir,
        } => {
            let out_dir = output_dir.unwrap_or_else(|| cfg.default_dir.clone());
            subsource::download_by_id(&cfg, movie_id, sub_id, &out_dir).await;
        }
        Commands::Key { action } => match action {
            KeyAction::Setup => cmd_key_setup(),
            KeyAction::Add { key } => cmd_key_add(key),
            KeyAction::Show => cmd_key_show(),
            KeyAction::Remove => cmd_key_remove(),
        },
        Commands::Config { action } => match action {
            ConfigAction::Set { key, value } => cfg.set_config(&key, &value),
            ConfigAction::Show => cmd_config_show(&cfg),
            ConfigAction::Reset => cfg.reset(),
        },
    }
}

fn cmd_key_setup() {
    let cfg = ConfigManager::load();
    if let Some(existing) = cfg.api_key() {
        println!("{}", "API key already configured:".cyan());
        println!("  {}", mask_key(&existing));
        println!();
        let mut answer = String::new();
        print!("Overwrite? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        std::io::stdin().read_line(&mut answer).unwrap();
        if answer.trim().to_lowercase() != "y" {
            println!("{}", "Cancelled".yellow());
            return;
        }
    }
    println!("{}", "Get your API key from:".cyan());
    println!("  1. Open https://subsource.net");
    println!("  2. Login or create account");
    println!("  3. Click Profile → API Key");
    println!("  4. Copy the key");
    println!();
    let mut key = String::new();
    print!("Enter API key: ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    std::io::stdin().read_line(&mut key).unwrap();
    let key = key.trim();
    if key.is_empty() {
        eprintln!("{}", "Error: Empty key".red());
        std::process::exit(1);
    }
    cfg.save_api_key(key);
    println!(
        "{}",
        format!("API key saved to {}", cfg.config_path.display()).green()
    );
}

fn cmd_key_add(key: Option<String>) {
    let cfg = ConfigManager::load();
    let key = match key {
        Some(k) => k,
        None => {
            let mut k = String::new();
            print!("Enter API key: ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            std::io::stdin().read_line(&mut k).unwrap();
            k.trim().to_string()
        }
    };
    if key.is_empty() {
        eprintln!("{}", "Error: Empty key".red());
        std::process::exit(1);
    }
    cfg.save_api_key(&key);
    println!("{}", "API key saved".green());
}

fn cmd_key_show() {
    let cfg = ConfigManager::load();
    match cfg.api_key() {
        Some(key) => {
            println!("{}: {}", "API Key".cyan(), mask_key(&key));
            println!("  (Stored in {})", cfg.config_path.display());
        }
        None => {
            println!("{}", "No API key configured".yellow());
        }
    }
}

fn cmd_key_remove() {
    let cfg = ConfigManager::load();
    if cfg.remove_api_key() {
        println!("{}", "API key removed".green());
    } else {
        println!("{}", "No API key to remove".yellow());
    }
}

fn cmd_config_show(cfg: &ConfigManager) {
    println!("{}", "Current config:".cyan());
    println!("  {}: {}", "Default language".bold(), cfg.default_lang);
    println!("  {}: {}", "Default output dir".bold(), cfg.default_dir);
    match cfg.api_key() {
        Some(key) => println!("  {}: {}", "API Key".bold(), mask_key(&key)),
        None => println!("  {}: {}", "API Key".bold(), "not set".yellow()),
    }
}

fn mask_key(key: &str) -> String {
    if key.len() <= 12 {
        return key.to_string();
    }
    format!("{}...{}", &key[..8], &key[key.len() - 4..])
}
