use anstyle::AnsiColor;
use clap::Parser;
use futures::{stream::FuturesUnordered, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    process,
    sync::{Arc, Mutex},
    time::Instant,
};
use strum::Display;
use subprocess::{Popen, PopenConfig, Redirection};

const BASE_URL: &str = "https://www.googleapis.com/webfonts/v1/webfonts";

#[derive(Display, Clone, Debug)]
#[strum(serialize_all = "kebab-case")]
enum FontStyles {
    Thin,
    ThinItalic,
    ExtraLight,
    ExtraLightItalic,
    Light,
    LightItalic,
    Regular,
    RegularItalic,
    Medium,
    MediumItalic,
    SemiBold,
    SemiBoldItalic,
    Bold,
    BoldItalic,
    ExtraBold,
    ExtraBoldItalic,
    Black,
    BlackItalic,
}

impl FontStyles {
    fn get_style_and_weight(&self) -> (&'static str, u16) {
        match self {
            FontStyles::Black => ("normal", 900),
            FontStyles::BlackItalic => ("italic", 900),
            FontStyles::ExtraBold => ("normal", 800),
            FontStyles::ExtraBoldItalic => ("italic", 800),
            FontStyles::Bold => ("normal", 700),
            FontStyles::BoldItalic => ("italic", 700),
            FontStyles::SemiBold => ("normal", 600),
            FontStyles::SemiBoldItalic => ("italic", 600),
            FontStyles::Medium => ("normal", 500),
            FontStyles::MediumItalic => ("italic", 500),
            FontStyles::Regular => ("normal", 400),
            FontStyles::RegularItalic => ("italic", 400),
            FontStyles::Light => ("normal", 300),
            FontStyles::LightItalic => ("italic", 300),
            FontStyles::ExtraLight => ("normal", 200),
            FontStyles::ExtraLightItalic => ("italic", 200),
            FontStyles::Thin => ("normal", 100),
            FontStyles::ThinItalic => ("italic", 100),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Font {
    items: Vec<FontFamily>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FontFamily {
    family: String,
    variants: Vec<String>,
    subsets: Vec<String>,
    files: HashMap<String, String>,
    category: String,
}

struct ProgressState {
    downloaded_count: u16,
    downloaded_files: Vec<FontStyles>,
}

// TODO: this isn't actually working, --help output is still not colored
fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(AnsiColor::Green.on_default())
        .header(AnsiColor::Cyan.on_default())
}

// TODO: Separate into commands := add, remove, compress (some people might prefer ttf idk)
// TODO: add, remove := specific weights, styles
// TODO: Add colors to CLI output
#[derive(Parser)]
#[command(name = "gfontapi")]
#[command(styles=get_styles())]
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

#[derive(Debug)]
enum ApiError {
    RequestFailed(reqwest::Error),
    BadStatus(StatusCode),
    ParseError(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::RequestFailed(err) => write!(f, "Request failed: {}", err),
            ApiError::BadStatus(status) => write!(f, "Bad status code: {}", status),
            ApiError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    // let start_time = Instant::now();

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

fn get_output_dir(target_dir: Option<PathBuf>) -> PathBuf {
    target_dir.unwrap_or_else(|| PathBuf::from("./fonts"))
}

async fn fetch_font_data(
    client: &Client,
    api_key: &str,
    font_name: &str,
) -> Result<FontFamily, Box<dyn std::error::Error>> {
    let api_url = format!(
        "{base_url}?key={key}&family={fontname}",
        base_url = BASE_URL,
        key = api_key,
        fontname = font_name
    );

    let response = client.get(&api_url).send().await.map_err(|err| {
        eprintln!(
            "{}: Failed to fetch `{}`\n  {}: {}",
            "error".red(),
            &api_url,
            "Caused by".red(),
            err
        );
        ApiError::RequestFailed(err)
    })?;

    if response.status() != StatusCode::OK {
        let status = response.status();
        eprintln!(
            "{}: Failed to fetch `{}`\n  {}: {}",
            "error".red(),
            &api_url,
            "Caused by".red(),
            status
        );
        return Err(Box::new(ApiError::BadStatus(status)));
    }

    let body = response.text().await?;
    let font_data: Font = serde_json::from_str(&body)
        .map_err(|_| ApiError::ParseError("Could not parse response".to_string()))?;

    Ok(font_data.items[0].clone())
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

    // Set up two different styles - one for progress and one for completion
    let progress_style = ProgressStyle::with_template("{spinner:.white} {msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");

    let completion_style = ProgressStyle::with_template("{msg}").unwrap();

    // Start with the progress style
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

    // Get the final count of downloaded files
    let downloaded_files = progress_state.lock().unwrap().downloaded_files.clone();
    let download_count = downloaded_files.len();

    // Calculate elapsed time
    let duration = start_time.elapsed();

    // Switch to the completion style (no spinner)
    spinner.set_style(completion_style);

    // Set the final message without emoji
    spinner.set_message(format!(
        "{}",
        format!(
            "Converted {} fonts in {:.2}s",
            download_count,
            duration.as_secs_f64()
        )
        .dimmed()
    ));

    // Finish but don't clear - we want the message to remain visible
    spinner.finish();

    Ok(downloaded_files)
}

fn write_css_file_for_font(
    font_styles: &[FontStyles],
    font_dir: &PathBuf,
    font_family_name: &str,
) -> Result<String, String> {
    let css_file_path = font_dir.join("fonts.css");
    let font_family_display_name =
        format_font_string(&font_dir.file_name().unwrap().to_string_lossy().to_string());

    for (idx, font_style) in font_styles.iter().enumerate() {
        let (font_style_name, font_weight) = font_style.get_style_and_weight();

        let font_face_string = format!(
            "@font-face {{\n\tfont-family: \"{}\";\n\tsrc: url({});\n\tfont-style: {};\n\tfont-weight: {};\n}}\n",
            &font_family_display_name,
            format!("{:?}", font_dir.join(format!("{}-{}.woff2", font_family_name, font_style))),
            font_style_name,
            font_weight
        );

        let mut file = if idx == 0 {
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&css_file_path)
        } else {
            OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(&css_file_path)
        }
        .map_err(|_| format!("Could not create file at path: {:?}", css_file_path))?;

        if let Err(e) = writeln!(file, "{}", font_face_string) {
            eprintln!(
                "{}: Could not write to file: {:?}\n  {}: {}",
                "error".red(),
                &css_file_path,
                "Caused by".red(),
                e
            )
        }
    }

    Ok(css_file_path.to_string_lossy().into())
}

fn format_font_string(input: &str) -> String {
    input
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first_char) => {
                    let upper_first_char = first_char.to_uppercase().collect::<String>();
                    upper_first_char + chars.as_str()
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
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

fn convert_to_woff2(ttf_path: &PathBuf) -> Result<(), String> {
    let mut process = Popen::create(
        &["./woff2_compress", &ttf_path.to_string_lossy()],
        PopenConfig {
            stdout: Redirection::Pipe,
            stderr: Redirection::Pipe,
            ..Default::default()
        },
    )
    .map_err(|_| "Failed to start woff2_compress".to_string())?;

    let status = process
        .wait()
        .map_err(|_| "Failed to wait for woff2_compress process".to_string())?;

    if !status.success() {
        return Err(format!("woff2_compress failed with status: {:?}", status));
    }

    std::fs::remove_file(ttf_path)
        .map_err(|_| format!("Could not delete file: {}", ttf_path.to_string_lossy()))?;

    Ok(())
}

fn get_api_key(cli_api_key: Option<String>) -> String {
    cli_api_key
        .or_else(|| env::var("GFONT_API_KEY").ok().filter(|key| !key.is_empty()))
        .unwrap_or_else(|| {
            eprintln!(
                "{}: Using gfontapi requires an API key.\
                \n  {}\n    - export GFONT_API_KEY={}\n    - gfontapi --api-key={}",
                "error".red(),
                "Pass it to the program in one of the following ways".dimmed(),
                "<YOUR_API_KEY>".cyan(),
                "<YOUR_API_KEY>".cyan()
            );
            process::exit(1);
        })
}

fn transpile_font_weight(font_string: &str) -> Result<FontStyles, String> {
    let font_weight_mappings: HashMap<&str, FontStyles> = HashMap::from([
        ("100", FontStyles::Thin),
        ("100italic", FontStyles::ThinItalic),
        ("200", FontStyles::ExtraLight),
        ("200italic", FontStyles::ExtraLightItalic),
        ("300", FontStyles::Light),
        ("300italic", FontStyles::LightItalic),
        ("regular", FontStyles::Regular),
        ("italic", FontStyles::RegularItalic),
        ("500", FontStyles::Medium),
        ("500italic", FontStyles::MediumItalic),
        ("600", FontStyles::SemiBold),
        ("600italic", FontStyles::SemiBoldItalic),
        ("700", FontStyles::Bold),
        ("700italic", FontStyles::BoldItalic),
        ("800", FontStyles::ExtraBold),
        ("800italic", FontStyles::ExtraBoldItalic),
        ("900", FontStyles::Black),
        ("900italic", FontStyles::BlackItalic),
    ]);

    font_weight_mappings
        .get(font_string)
        .cloned()
        .ok_or_else(|| "Couldn't find the variant in the hashmap".to_owned())
}
