use anstyle::AnsiColor;
use clap::Parser;
use futures::{stream::FuturesUnordered, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::Write,
    path::PathBuf,
    process,
    sync::{Arc, Mutex},
    time::Instant,
};
use strum::Display;
use subprocess::{Popen, PopenConfig, Redirection};

const BASE_URL: &str = "https://www.googleapis.com/webfonts/v1/webfonts";

#[derive(Display, Clone)]
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

#[derive(Debug, Serialize, Deserialize)]
struct Font {
    items: Vec<FontFamily>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FontFamily {
    family: String,
    variants: Vec<String>,
    subsets: Vec<String>,
    files: HashMap<String, String>,
    category: String,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let start_time = Instant::now();

    let output_dir: PathBuf;
    if let Some(path) = args.target_dir {
        output_dir = path
    } else {
        output_dir = PathBuf::from("./fonts")
    };
    let api_key = get_api_key(args.api_key);
    let api_url = format!(
        "{base_url}?key={key}&family={fontname}",
        base_url = BASE_URL,
        key = api_key,
        fontname = args.fontname
    );
    let client = reqwest::Client::builder().build()?;

    let response = client.get(&api_url).send().await;
    if let Err(err) = response {
        eprintln!(
            "{}: Failed to fetch `{}`\n  {}: {}",
            "error".red(),
            &api_url,
            "Caused by".red(),
            err
        );
        process::exit(1);
    }
    let response = response?;
    if response.status() != StatusCode::OK {
        eprintln!(
            "{}: Failed to fetch `{}`\n  {}: {}",
            "error".red(),
            &api_url,
            "Caused by".red(),
            response.status()
        );
        process::exit(1);
    };
    let body = response.text().await?;
    let val: Font = serde_json::from_str(&body).or(Err(format!(
        "Could not parse response into appropriate Font"
    )))?;

    let font_family = &val.items[0];
    let mut download_tasks = FuturesUnordered::new();
    let family_name = font_family.family.to_lowercase().replace(' ', "-");
    let files_download_dir = output_dir.join(family_name.to_lowercase());
    let total_files = font_family.files.len();
    let downloaded_count = Arc::new(Mutex::new(0));
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.white} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    println!(
        "Creating font directory at: {}",
        &files_download_dir.to_string_lossy().cyan()
    );
    std::fs::create_dir_all(&files_download_dir)?;
    for (variant, url) in &font_family.files {
        let font_style = transpile_font_weight(&variant.clone()).or(Err(format!(
            "Couldn't find variant mapping for {}",
            variant
        )))?;
        let download_url = url.clone();
        // TOOD: this is really really bad, fix this
        let family_name_clone = family_name.clone();
        let files_download_dir_clone = files_download_dir.clone();
        let client_clone = client.clone();
        let downloaded_count_clone = Arc::clone(&downloaded_count);
        let spinner_clone = spinner.clone();
        // let mp_clone = Arc::clone(&mp);
        let task = tokio::spawn(async move {
            // let pb = mp_clone.add(ProgressBar::new(100));
            // pb.set_style(
            //     ProgressStyle::with_template("{msg:10.dim} {bar:30.green/dim}")
            //         .unwrap()
            //         .progress_chars("--"),
            // );
            // pb.set_message(format!("{}=={}", family_name_clone, font_style.dimmed()));
            download_font_file(
                &client_clone,
                &download_url,
                family_name_clone,
                font_style,
                files_download_dir_clone,
                // pb,
            )
            .await
            .unwrap();
            let mut count = downloaded_count_clone.lock().unwrap();
            *count += 1;
            spinner_clone.set_message(format!("Downloading fonts ({}/{})", *count, total_files));
            spinner_clone.inc(1);
        });
        download_tasks.push(task);
    }
    while let Some(result) = download_tasks.next().await {
        result.unwrap();
    }
    // sleep(Duration::from_millis(100)).await;
    let duration = start_time.elapsed();
    let message = format!("Installed {} font files in {:.2?}", total_files, duration);
    spinner.finish_with_message(format!("{}", message.dimmed()));
    spinner.set_style(ProgressStyle::with_template("{msg}").unwrap());
    spinner.finish();

    // Prints all the files that have just been added in this format
    for (variant, _) in &font_family.files {
        let font_style = transpile_font_weight(&variant).unwrap();
        println!(
            " {} {}{}",
            "+".green(),
            &family_name,
            format!("=={}", &font_style).dimmed()
        );
    }
    Ok(())
}

async fn download_font_file(
    client: &Client,
    url: &str,
    family_name: String,
    font_style: FontStyles,
    files_download_dir: PathBuf,
    // progress_bar: ProgressBar,
) -> Result<(), String> {
    let output_path = files_download_dir.join(format!("{}-{}.ttf", family_name, &font_style));
    let response = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from {}", &url)))?;
    // let total_size = response.content_length().unwrap_or(0);
    let mut file = File::create(&output_path).or(Err(format!(
        "Failed to create file at: {}",
        &output_path.to_string_lossy()
    )))?;
    // let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    while let Some(item) = stream.next().await {
        let chunk = item.or(Err("Error while downloading file".to_string()))?;
        file.write_all(&chunk).or(Err(format!(
            "Error while writing chunk {:?} to file {}",
            &chunk,
            output_path.to_string_lossy()
        )))?;
    }
    // TODO: This should also be its own function
    // TODO: ideally this should download and build the woff2_compress binary if it doesn't exist
    // and then run it on the files instead of shipping with it by default
    let mut process = Popen::create(
        &["./woff2_compress", &output_path.to_string_lossy()],
        PopenConfig {
            stdout: Redirection::Pipe,
            stderr: Redirection::Pipe,
            ..Default::default()
        },
    )
    .unwrap();
    // Makes sure we don't exit the thread before the subprocess has returned
    // Not sure why that isn't the default behavior
    let status = process.wait().unwrap();
    if !status.success() {
        return Err(format!("woff2_compress failed with status: {:?}", status));
    }
    std::fs::remove_file(&output_path).or(Err(format!(
        "Could not delete file: {}",
        &output_path.to_string_lossy()
    )))?;
    Ok(())
}

fn get_api_key(cli_api_key: Option<String>) -> String {
    // TODO: Not sure if this is the most idiomatic way to do this.
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
    let font_weight_mappings: HashMap<String, FontStyles> = HashMap::from([
        (String::from("100"), FontStyles::Thin),
        (String::from("100italic"), FontStyles::ThinItalic),
        (String::from("200"), FontStyles::ExtraLight),
        (String::from("200italic"), FontStyles::ExtraLightItalic),
        (String::from("300"), FontStyles::Light),
        (String::from("300italic"), FontStyles::LightItalic),
        (String::from("regular"), FontStyles::Regular),
        (String::from("italic"), FontStyles::RegularItalic),
        (String::from("500"), FontStyles::Medium),
        (String::from("500italic"), FontStyles::MediumItalic),
        (String::from("600"), FontStyles::SemiBold),
        (String::from("600italic"), FontStyles::SemiBoldItalic),
        (String::from("700"), FontStyles::Bold),
        (String::from("700italic"), FontStyles::BoldItalic),
        (String::from("800"), FontStyles::ExtraBold),
        (String::from("800italic"), FontStyles::ExtraBoldItalic),
        (String::from("900"), FontStyles::Black),
        (String::from("900italic"), FontStyles::BlackItalic),
    ]);
    if let Some(res) = font_weight_mappings.get(font_string) {
        Ok(res.clone())
    } else {
        Err("Couldn't find the variant in the hashmap".to_owned())
    }
}
