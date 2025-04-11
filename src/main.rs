pub mod fonts;
pub mod utils;

use clap::Parser;
use fonts::{fetch_font_data, transpile_font_weight, FontFamily, FontStyles};
use futures::{stream::FuturesUnordered, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use reqwest::Client;
use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};
use utils::{convert_to_woff2, get_api_key, get_output_dir, write_css_file_for_font};

const BASE_URL: &str = "https://www.googleapis.com/webfonts/v1/webfonts";

struct ProgressState {
    downloaded_count: u16,
    downloaded_files: Vec<FontStyles>,
}

// TODO: Separate into commands := add, remove, compress (some people might prefer ttf idk)
// TODO: add, remove := specific weights, styles
// TODO: Add colors to CLI output
#[derive(Parser)]
#[command(name = "gfontapi")]
#[command(version = "0.1.0")]
#[command(about = "Manage all your google fonts from the terminal.")]
#[command(
    help_template = "{about}\n\nUsage: {name} [OPTIONS] \"[fontname]\"\n\nOptions\n{options}"
)]
struct Args {
    /// Name of the font to download
    #[arg(value_name = "fontname")]
    fontname: String,
    /// Directory to place the converted fonts
    #[arg(
        short,
        long = "target-dir",
        help_heading = "options",
        help = "target directory, defaults to ./fonts."
    )]
    target_dir: Option<PathBuf>,
    /// Users google application API key
    #[arg(
        short,
        long = "api-key",
        help_heading = "options",
        help = "google api key generated from developer console, can also be set as `EXPORT GFONT_API_KEY=<API_KEY>`"
    )]
    api_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let output_dir = get_output_dir(args.target_dir);
    let api_key = get_api_key(args.api_key);
    let client = reqwest::Client::builder().build()?;

    let font_family = fetch_font_data(&client, &api_key, &args.fontname).await?;
    let family_name = font_family.family.to_lowercase().replace(' ', "-");
    let font_dir = output_dir.join(&family_name);

    println!(
        "Creating font directory at: {}",
        &font_dir.to_string_lossy().cyan()
    );
    std::fs::create_dir_all(&font_dir)?;

    let download_results =
        download_font_files(&client, &font_family, &family_name, &font_dir).await?;

    println!(
        "{} {}",
        "Writing fonts.css file for".dimmed(),
        &family_name.cyan()
    );

    match write_css_file_for_font(&download_results, &font_dir, &family_name) {
        Err(err) => eprintln!(
            "{}: Failed to write fonts file\n  {}: {}",
            "error".red(),
            "Caused by".red(),
            err
        ),
        Ok(file_path) => println!(
            "{} {}",
            "Finished writing fonts.css file to".dimmed(),
            &file_path.dimmed()
        ),
    }

    for font_style in &download_results {
        println!(
            " {} {}{}",
            "+".green(),
            &family_name,
            format!("=={}", &font_style).dimmed()
        );
    }

    Ok(())
}

async fn download_font_files(
    client: &Client,
    font_family: &FontFamily,
    family_name: &str,
    output_dir: &PathBuf,
) -> Result<Vec<FontStyles>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let total_files = font_family.files.len();
    let progress_state = Arc::new(Mutex::new(ProgressState {
        downloaded_count: 0,
        downloaded_files: vec![],
    }));

    let spinner = ProgressBar::new_spinner();

    let progress_style = ProgressStyle::with_template("{spinner:.white} {msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");

    let completion_style = ProgressStyle::with_template("{msg.dimmed()}").unwrap();

    spinner.set_style(progress_style);

    let mp = Arc::new(MultiProgress::new());
    let spinner = mp.add(spinner);

    let mut download_tasks = FuturesUnordered::new();

    for (variant, url) in &font_family.files {
        let font_style = transpile_font_weight(variant)
            .map_err(|e| format!("Couldn't find variant mapping for {}: {}", variant, e))?;

        let download_url = url.to_string();
        let progress_state_clone = Arc::clone(&progress_state);
        let spinner_clone = spinner.clone();
        let mp_clone = Arc::clone(&mp);
        let family_name_str = family_name.to_string();
        let client_clone = client.clone();
        let output_path = output_dir.join(format!("{}-{}.ttf", family_name, font_style));

        let task = tokio::spawn(async move {
            let pb = mp_clone.add(ProgressBar::new(100));
            pb.set_style(
                ProgressStyle::with_template("{msg:10.dim} {bar:30.green/dim}")
                    .unwrap()
                    .progress_chars("--"),
            );
            pb.set_message(format!("{}=={}", family_name_str, font_style.dimmed()));
            let result =
                download_font_file(&client_clone, &download_url, &output_path, pb.clone()).await;
            pb.finish_and_clear();
            convert_to_woff2(&output_path)?;

            let mut progress_state = progress_state_clone.lock().unwrap();
            progress_state.downloaded_count += 1;
            if result.is_ok() {
                progress_state.downloaded_files.push(font_style);
            }

            // Update the spinner message with the current progress
            spinner_clone.set_message(format!(
                "Converting fonts... ({}/{})",
                progress_state.downloaded_count, total_files
            ));

            result
        });

        download_tasks.push(task);
    }

    while let Some(result) = download_tasks.next().await {
        if let Err(e) = result {
            eprintln!("Task error: {}", e);
        } else if let Ok(Err(e)) = result {
            eprintln!("Download error: {}", e);
        }
    }

    let downloaded_files = progress_state.lock().unwrap().downloaded_files.clone();
    let download_count = downloaded_files.len();

    let duration = start_time.elapsed();

    spinner.set_style(completion_style);

    spinner.set_message(format!(
        "Converted {} fonts in {:.2}s",
        download_count,
        duration.as_secs_f64() // )
    ));

    spinner.finish();

    Ok(downloaded_files)
}

async fn download_font_file(
    client: &Client,
    url: &str,
    output_path: &PathBuf,
    progress_bar: ProgressBar,
) -> Result<(), String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| format!("Failed to GET from {}", url))?;

    let total_size = response.content_length().unwrap_or(0);
    progress_bar.set_length(total_size);

    let mut file = File::create(&output_path).map_err(|_| {
        format!(
            "Failed to create file at: {}",
            output_path.to_string_lossy()
        )
    })?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|_| "Error while downloading file".to_string())?;
        file.write_all(&chunk).map_err(|_| {
            format!(
                "Error while writing to file {}",
                output_path.to_string_lossy()
            )
        })?;

        downloaded += chunk.len() as u64;
        progress_bar.set_position(downloaded);
    }

    // Don't finish or clear here - let the calling function handle it
    // This ensures proper coordination with the MultiProgress instance
    Ok(())
}
