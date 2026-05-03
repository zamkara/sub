use colored::Colorize;
use reqwest::Client;
use serde::Deserialize;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::config::ConfigManager;
use crate::mkv::{lang_name, lang_to_api};

const API_BASE: &str = "https://api.subsource.net/api/v1";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiResponse<T> {
    data: Option<Vec<T>>,
}

#[derive(Debug, Deserialize)]
struct Movie {
    #[serde(rename = "movieId")]
    movie_id: u64,
    title: String,
    #[serde(rename = "releaseYear")]
    release_year: Option<u32>,
    #[serde(rename = "type")]
    media_type: String,
}

#[derive(Debug, Deserialize)]
struct Subtitle {
    #[serde(rename = "subtitleId")]
    subtitle_id: u64,
    language: String,
    #[serde(rename = "releaseInfo")]
    release_info: Vec<String>,
    #[serde(rename = "hearingImpaired")]
    hearing_impaired: bool,
}

pub async fn search(
    cfg: &ConfigManager,
    query: &str,
    language: &str,
    year: Option<u32>,
    output_dir: &str,
    non_interactive: bool,
    verbose: bool,
) {
    let api_key = match cfg.api_key() {
        Some(k) => k,
        None => {
            eprintln!("{}", "Error: API key not configured".red());
            eprintln!("Run {} to configure", "sub key setup".cyan());
            std::process::exit(1);
        }
    };

    fs::create_dir_all(output_dir).ok();

    let api_lang = lang_to_api(language);

    println!(
        "{} {}{}",
        "Searching:".cyan(),
        query.green(),
        year.map(|y| format!(" ({})", y).yellow())
            .unwrap_or_default()
    );

    let client = Client::new();

    let movies = match search_movies(&client, &api_key, query, year, verbose).await {
        Some(m) => m,
        None => return,
    };

    if movies.is_empty() {
        println!("{}", "No results found".yellow());
        return;
    }

    let selected = if non_interactive {
        &movies[0]
    } else {
        display_movies(&movies);
        match pick_number(movies.len(), "Select [1-N]", 1) {
            Some(n) => &movies[n - 1],
            None => {
                println!("{}", "Skipped".yellow());
                return;
            }
        }
    };

    println!(
        "{} {} ({})",
        "Selected:".cyan(),
        selected.title.green(),
        selected.release_year.unwrap_or(0)
    );

    println!("{} {}", "Fetching subtitles:".cyan(), api_lang.green());

    let subs = match get_subtitles(&client, &api_key, selected.movie_id, &api_lang, verbose).await {
        Some(s) => s,
        None => return,
    };

    if subs.is_empty() {
        println!("{}", "No subtitles found".yellow());
        return;
    }

    let mut sorted_subs = subs;
    sort_by_priority(&mut sorted_subs, language);

    if non_interactive {
        let sub = &sorted_subs[0];
        println!(
            "{} {}",
            "Auto-selecting:".cyan(),
            sub.release_info.first().cloned().unwrap_or_default()
        );
        download_and_extract(&client, &api_key, sub.subtitle_id, output_dir).await;
    } else {
        display_subtitles(&sorted_subs, language);
        let input = pick_input("Select (e.g. 1,3,5 or 1-3 or 'all')");
        let selected_ids = parse_selection(&input, sorted_subs.len());
        if selected_ids.is_empty() {
            println!("{}", "No subtitles selected".yellow());
            return;
        }

        let total = selected_ids.len();
        println!("{}", format!("Downloading {} subtitle(s)...", total).cyan());

        for (i, &idx) in selected_ids.iter().enumerate() {
            let sub = &sorted_subs[idx];
            let fname = lang_name(&sub.language);
            let rel = sub.release_info.first().cloned().unwrap_or_default();
            println!(
                "\n{} {} {}",
                format!("[{}/{}]", i + 1, total).green(),
                rel.green(),
                format!("[{}]", fname).yellow()
            );

            download_and_extract(&client, &api_key, sub.subtitle_id, output_dir).await;
        }

        println!(
            "\n{}",
            format!("Done: {}/{} subtitle(s) downloaded", total, total).green()
        );
    }
}

pub async fn download_by_id(cfg: &ConfigManager, _movie_id: u64, sub_id: u64, output_dir: &str) {
    let api_key = match cfg.api_key() {
        Some(k) => k,
        None => {
            eprintln!("{}", "Error: API key not configured".red());
            eprintln!("Run {} to configure", "sub key setup".cyan());
            std::process::exit(1);
        }
    };

    fs::create_dir_all(output_dir).ok();
    let client = Client::new();
    download_and_extract(&client, &api_key, sub_id, output_dir).await;
}

async fn search_movies(
    client: &Client,
    api_key: &str,
    query: &str,
    year: Option<u32>,
    verbose: bool,
) -> Option<Vec<Movie>> {
    let mut url = format!(
        "{}/movies/search?searchType=text&q={}",
        API_BASE,
        urlencoding(query)
    );
    if let Some(y) = year {
        url.push_str(&format!("&year={}", y));
    }

    let resp = client
        .get(&url)
        .header("X-API-Key", api_key)
        .header("Accept", "application/json")
        .send()
        .await;

    match resp {
        Ok(r) => {
            if verbose {
                let text = r.text().await.unwrap_or_default();
                println!("{} {}", "Search response:".cyan(), &text);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        return serde_json::from_value(serde_json::Value::Array(data.clone())).ok();
                    }
                }
            } else {
                let text = r.text().await.unwrap_or_default();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        return serde_json::from_value(serde_json::Value::Array(data.clone())).ok();
                    }
                }
            }
            None
        }
        Err(e) => {
            eprintln!("{}", format!("Error: {}", e).red());
            None
        }
    }
}

async fn get_subtitles(
    client: &Client,
    api_key: &str,
    movie_id: u64,
    language: &str,
    verbose: bool,
) -> Option<Vec<Subtitle>> {
    let url = format!(
        "{}/subtitles?movieId={}&language={}",
        API_BASE, movie_id, language
    );

    let resp = client
        .get(&url)
        .header("X-API-Key", api_key)
        .header("Accept", "application/json")
        .send()
        .await;

    match resp {
        Ok(r) => {
            let status = r.status();
            if !status.is_success() {
                eprintln!("{}", format!("Error: HTTP {}", status).red());
                if verbose {
                    let text = r.text().await.unwrap_or_default();
                    eprintln!("{}", text);
                }
                return None;
            }
            let text = r.text().await.unwrap_or_default();
            if verbose {
                println!("{} {}", "Subtitles response:".cyan(), &text);
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                    return serde_json::from_value(serde_json::Value::Array(data.clone())).ok();
                }
            }
            None
        }
        Err(e) => {
            eprintln!("{}", format!("Error: {}", e).red());
            None
        }
    }
}

async fn download_and_extract(client: &Client, api_key: &str, sub_id: u64, output_dir: &str) {
    let url = format!("{}/subtitles/{}/download", API_BASE, sub_id);

    let resp = client.get(&url).header("X-API-Key", api_key).send().await;

    match resp {
        Ok(r) => {
            if !r.status().is_success() {
                eprintln!("{}", "Error: Download failed".red());
                return;
            }
            let bytes = r.bytes().await.unwrap_or_default();
            if bytes.is_empty() {
                eprintln!("{}", "Error: Empty download".red());
                return;
            }

            if let Some(srt_path) = extract_srt(&bytes, output_dir) {
                if let Ok(meta) = fs::metadata(&srt_path) {
                    let size = meta.len();
                    println!(
                        "{}",
                        format!("Saved: {} ({:.1} KB)", srt_path, size as f64 / 1024.0).green()
                    );
                }
            } else {
                eprintln!("{}", "Error: No SRT file in download".red());
            }
        }
        Err(e) => {
            eprintln!("{}", format!("Error: {}", e).red());
        }
    }
}

fn extract_srt(data: &[u8], output_dir: &str) -> Option<String> {
    if data.len() >= 4 && data[0] == 0x50 && data[1] == 0x4B && data[2] == 0x03 && data[3] == 0x04 {
        let tmpdir = tempfile::tempdir().ok()?;
        let zip_path = tmpdir.path().join("sub.zip");
        fs::write(&zip_path, data).ok()?;

        let reader = fs::File::open(&zip_path).ok()?;
        let mut archive = zip::ZipArchive::new(reader).ok()?;

        let mut srt_names: Vec<String> = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                if name.to_lowercase().ends_with(".srt") && !name.contains("__MACOSX") {
                    srt_names.push(name);
                }
            }
        }

        if !srt_names.is_empty() {
            let chosen = srt_names.remove(0);

            let mut file = archive.by_name(&chosen).ok()?;
            let dest_path = Path::new(output_dir).join(&chosen);
            let dest_dir = dest_path.parent()?;
            fs::create_dir_all(dest_dir).ok();

            let mut content = Vec::new();
            io::copy(&mut file, &mut content).ok()?;
            fs::write(&dest_path, content).ok()?;

            return Some(dest_path.to_string_lossy().to_string());
        }
    } else {
        let srt_path = Path::new(output_dir).join("subtitle.srt");
        fs::write(&srt_path, data).ok()?;
        return Some(srt_path.to_string_lossy().to_string());
    }
    None
}

fn sort_by_priority(subs: &mut [Subtitle], default_lang: &str) {
    let api_dl = lang_to_api(default_lang).to_lowercase();
    subs.sort_by(|a, b| {
        let pa = priority(&a.language, &api_dl);
        let pb = priority(&b.language, &api_dl);
        pa.cmp(&pb)
    });
}

fn priority(lang: &str, default_lang: &str) -> u8 {
    let l = lang.to_lowercase();
    if default_lang.contains(&l) || l.contains(default_lang) {
        return 0;
    }
    if l.contains("english") || l.contains("eng") {
        return 1;
    }
    2
}

fn display_movies(movies: &[Movie]) {
    println!("{}", "Search results:".cyan());
    for (i, m) in movies.iter().enumerate() {
        let year = m
            .release_year
            .map(|y| y.to_string())
            .unwrap_or("?".to_string());
        println!(
            "  {} {} ({}) - {}",
            format!("[{}]", i + 1).green(),
            m.title,
            year,
            m.media_type
        );
    }
    println!();
}

fn display_subtitles(subs: &[Subtitle], default_lang: &str) {
    let api_dl = lang_to_api(default_lang);
    println!("{} ({})", "Available subtitles:".cyan(), subs.len());
    println!(
        "{}",
        format!("{:<4} {:<12} {:<14}", "#", "Language", "Release").cyan()
    );
    println!(
        "{}",
        format!("{:<4} {:<12} {:<14}", "---", "--------", "-------").cyan()
    );

    for (i, s) in subs.iter().enumerate() {
        let fname = lang_name(&s.language);
        let api_lang = lang_to_api(&s.language);
        let marker = if api_lang == api_dl {
            "★ ".green().to_string()
        } else {
            "".to_string()
        };
        let hi_tag = if s.hearing_impaired {
            " [HI]".yellow().to_string()
        } else {
            "".to_string()
        };
        let rel = s.release_info.first().cloned().unwrap_or_default();
        println!("{}{:<4} {:<12} {:<14}{}", marker, i + 1, fname, rel, hi_tag);
    }
    println!();
}

fn pick_number(max: usize, prompt: &str, default: usize) -> Option<usize> {
    let input = pick_input(&format!("{} (default {})", prompt, default));
    if input.trim().is_empty() {
        return Some(default);
    }
    if input.trim().to_lowercase() == "s" {
        return None;
    }
    input
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|&n| n >= 1 && n <= max)
}

fn pick_input(prompt: &str) -> String {
    print!("{}: ", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input
}

fn parse_selection(input: &str, max: usize) -> Vec<usize> {
    let input = input.trim().to_lowercase();
    if input == "all" {
        return (0..max).collect();
    }

    let mut indices = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                {
                    for i in start..=end {
                        if i >= 1 && i <= max {
                            indices.push(i - 1);
                        }
                    }
                }
            }
        } else if let Ok(n) = part.parse::<usize>() {
            if n >= 1 && n <= max {
                indices.push(n - 1);
            }
        }
    }
    indices.sort();
    indices.dedup();
    indices
}

fn urlencoding(s: &str) -> String {
    let mut encoded = String::new();
    for c in s.chars() {
        match c {
            ' ' => encoded.push_str("%20"),
            c if c.is_ascii_alphanumeric() || "-_.~".contains(c) => encoded.push(c),
            c => {
                for byte in c.to_string().as_bytes() {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    encoded
}
