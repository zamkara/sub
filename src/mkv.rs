use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn list_subtitles(video: &str, default_lang: &str) {
    if !Path::new(video).exists() {
        eprintln!("{}", format!("Error: File not found: {}", video).red());
        std::process::exit(1);
    }

    let api_default = lang_to_api(default_lang);

    println!(
        "{} {}",
        "Subtitle tracks in:".cyan(),
        basename(video).green()
    );
    println!(
        "{}",
        format!("{:<8} {:<14} {:<16} {}", "Track", "Type", "Language", "Name").cyan()
    );
    println!(
        "{}",
        format!("{:<8} {:<14} {:<16} {}", "-----", "----", "--------", "----").cyan()
    );

    let output = Command::new("mkvmerge")
        .args([
            "--identification-format",
            "json",
            "--identify",
            video,
        ])
        .output()
        .expect("Failed to run mkvmerge");

    if !output.status.success() {
        eprintln!("mkvmerge failed");
        std::process::exit(1);
    }

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Invalid JSON from mkvmerge");

    let mut found = false;
    if let Some(tracks) = json.get("tracks").and_then(|t| t.as_array()) {
        for track in tracks {
            if track.get("type").and_then(|t| t.as_str()) == Some("subtitles") {
                found = true;
                let id = track.get("id").and_then(|t| t.as_u64()).unwrap_or(0);
                let props = track.get("properties").cloned().unwrap_or_default();
                let lang = props
                    .get("language")
                    .and_then(|l| l.as_str())
                    .unwrap_or("und");
                let name = props
                    .get("track_name")
                    .and_then(|l| l.as_str())
                    .unwrap_or("");

                let fname = lang_name(lang);
                let api_lang = lang_to_api(lang);
                let marker = if api_lang == api_default
                    || api_default.starts_with(&api_lang[..3.min(api_lang.len())])
                {
                    "★ ".green().to_string()
                } else {
                    "".to_string()
                };

                println!(
                    "{}{:<8} {:<14} {:<16} {}",
                    marker,
                    id,
                    "subtitles",
                    format!("{} ({})", lang, fname),
                    name
                );
            }
        }
    }

    if !found {
        println!("{}", "No subtitle tracks found.".yellow());
    }
}

pub fn extract_subtitle(video: &str, track_id: usize, output: Option<&str>) {
    if !Path::new(video).exists() {
        eprintln!("{}", format!("Error: File not found: {}", video).red());
        std::process::exit(1);
    }

    let output_path: String = match output {
        Some(o) => o.to_string(),
        None => {
            let base = video.trim_end_matches(|c| c != '.').trim_end_matches('.');
            let lang = get_track_language(video, track_id);
            let fname = lang_name(&lang);
            format!("{}_{}_{}.srt", base, fname, track_id)
        }
    };

    println!(
        "{} {} -> {}",
        format!("Extracting track {}", track_id).cyan(),
        track_id.to_string().green(),
        output_path.green()
    );

    let status = Command::new("mkvextract")
        .args(["tracks", video, &format!("{}:{}", track_id, output_path)])
        .status()
        .expect("Failed to run mkvextract");

    if !status.success() {
        eprintln!("{}", "Error: Extraction failed".red());
        std::process::exit(1);
    }

    if let Ok(meta) = fs::metadata(&output_path) {
        let size = meta.len();
        println!(
            "{}",
            format!("Done: {} ({:.1} KB)", output_path, size as f64 / 1024.0).green()
        );
    }
}

pub fn inject_subtitle(video: &str, subtitle: &str, lang: Option<&str>, name: Option<&str>) {
    if !Path::new(video).exists() {
        eprintln!("{}", format!("Error: File not found: {}", video).red());
        std::process::exit(1);
    }
    if !Path::new(subtitle).exists() {
        eprintln!("{}", format!("Error: File not found: {}", subtitle).red());
        std::process::exit(1);
    }

    let lang_code = lang.unwrap_or("ind");
    let norm_lang = normalize_lang(lang_code);
    let track_name = name.unwrap_or(&lang_name(&norm_lang)).to_string();

    let tmp_path = format!("{}_tmp_subinject.mkv", video.trim_end_matches(".mkv"));

    println!(
        "{} {} {} {} {}",
        "Injecting".cyan(),
        basename(subtitle).green(),
        "as".cyan(),
        format!("{} ({})", track_name, norm_lang).green(),
        format!("into {}", basename(video)).cyan()
    );

    let status = Command::new("mkvmerge")
        .args([
            "-o",
            &tmp_path,
            "--language",
            &format!("0:{}", norm_lang),
            "--track-name",
            &format!("0:{}", track_name),
            subtitle,
            video,
        ])
        .status()
        .expect("Failed to run mkvmerge");

    if !status.success() {
        eprintln!("{}", "Error: Injection failed".red());
        std::process::exit(1);
    }

    let backup = format!("{}_backup.mkv", video.trim_end_matches(".mkv"));
    fs::rename(video, &backup).expect("Failed to create backup");
    fs::rename(&tmp_path, video).expect("Failed to replace video");
    println!(
        "{}",
        format!(
            "Done: Subtitle injected. Backup saved as {}",
            backup
        )
        .green()
    );
}

fn get_track_language(video: &str, track_id: usize) -> String {
    let output = Command::new("mkvmerge")
        .args([
            "--identification-format",
            "json",
            "--identify",
            video,
        ])
        .output()
        .ok()
        .map(|o| o.stdout)
        .unwrap_or_default();

    let json: serde_json::Value = serde_json::from_slice(&output).ok().unwrap_or_default();
    if let Some(tracks) = json.get("tracks").and_then(|t| t.as_array()) {
        for track in tracks {
            if track.get("id").and_then(|t| t.as_u64()) == Some(track_id as u64) {
                if let Some(lang) = track
                    .get("properties")
                    .and_then(|p| p.get("language"))
                    .and_then(|l| l.as_str())
                {
                    return lang.to_string();
                }
            }
        }
    }
    "und".to_string()
}

fn basename(path: &str) -> &str {
    Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path)
}

pub fn lang_name(lang: &str) -> String {
    match lang.to_lowercase().as_str() {
        "ind" | "id" | "in" | "indonesian" => "Indonesian",
        "eng" | "en" | "english" => "English",
        "fre" | "fr" | "french" => "French",
        "spa" | "es" | "spanish" => "Spanish",
        "jpn" | "ja" | "japanese" => "Japanese",
        "kor" | "ko" | "korean" => "Korean",
        "zho" | "zh" | "chinese" => "Chinese",
        "may" | "ms" | "malay" => "Malay",
        "tha" | "th" | "thai" => "Thai",
        "vie" | "vi" | "vietnamese" => "Vietnamese",
        "ara" | "ar" | "arabic" => "Arabic",
        "por" | "pt" | "portuguese" => "Portuguese",
        "deu" | "de" | "german" => "German",
        "ita" | "it" | "italian" => "Italian",
        "rus" | "ru" | "russian" => "Russian",
        _ => lang,
    }
    .to_string()
}

pub fn lang_to_api(lang: &str) -> String {
    match lang.to_lowercase().as_str() {
        "indonesian" | "ind" | "id" | "in" => "indonesian",
        "english" | "eng" | "en" => "english",
        "french" | "fre" | "fr" => "french",
        "spanish" | "spa" | "es" => "spanish",
        "japanese" | "jpn" | "ja" => "japanese",
        "korean" | "kor" | "ko" => "korean",
        "chinese" | "zho" | "zh" => "chinese",
        "malay" | "may" | "ms" => "malay",
        "thai" | "tha" | "th" => "thai",
        "vietnamese" | "vie" | "vi" => "vietnamese",
        "arabic" | "ara" | "ar" => "arabic",
        "portuguese" | "por" | "pt" => "portuguese",
        "german" | "deu" | "de" => "german",
        "italian" | "ita" | "it" => "italian",
        "russian" | "rus" | "ru" => "russian",
        _ => lang,
    }
    .to_string()
}

pub fn normalize_lang(lang: &str) -> String {
    match lang.to_lowercase().as_str() {
        "indonesian" | "id" | "in" | "ind" => "ind",
        "english" | "en" | "eng" => "eng",
        "french" | "fr" | "fre" => "fre",
        "spanish" | "es" | "spa" => "spa",
        "japanese" | "ja" | "jpn" => "jpn",
        "korean" | "ko" | "kor" => "kor",
        "chinese" | "zh" | "zho" => "zho",
        "malay" | "ms" | "may" => "may",
        "thai" | "th" | "tha" => "tha",
        "vietnamese" | "vi" | "vie" => "vie",
        "arabic" | "ar" | "ara" => "ara",
        "portuguese" | "pt" | "por" => "por",
        "german" | "de" | "deu" => "deu",
        "italian" | "it" | "ita" => "ita",
        "russian" | "ru" | "rus" => "rus",
        _ => lang,
    }
    .to_string()
}
