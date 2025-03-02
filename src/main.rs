use anstyle::AnsiColor;
use clap::Parser;
use futures::{stream::FuturesUnordered, StreamExt};
use owo_colors::OwoColorize;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs::File, io::Write, path::PathBuf, process};
use strum::Display;
use subprocess::{Popen, PopenConfig};

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

// struct ProgressState {
//     total: usize,
//     completed: usize,
//     downloaded_files: Vec<String>,
// }

// TODO: Separate into commands := add, remove, compress (some people might prefer ttf idk)
// TODO: add := specific weights, styles
// TODO: remove := specific weights, styles
// TODO: Add colors to CLI output (check how @uv has done it)
#[derive(Parser)]
#[command(name = "gfontapi")]
#[command(styles=get_styles())]
#[command(version = "0.1.0")]
#[command(about = "Manage all your google fonts from the terminal.")]
#[command(help_template = "{about}\n\nUsage: {name} [OPTIONS] [fontname]\n\nOptions\n{options}")]
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
    // let start_time = Instant::now();

    let output_dir: PathBuf;
    if let Some(path) = args.target_dir {
        output_dir = path
    } else {
        output_dir = PathBuf::from("./fonts")
    };
    let api_key = get_api_key(args.api_key);
    println!("Using API key: {}", &api_key.cyan());
    let api_url = format!(
        "{base_url}?key={key}&family={fontname}",
        base_url = BASE_URL,
        key = api_key,
        fontname = args.fontname
    );
    let client = reqwest::Client::builder().build()?;

    let response = client.get(&api_url).send().await?;
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
    let family_name = font_family.family.to_lowercase();
    let files_download_dir = output_dir.join(family_name.to_lowercase());
    println!(
        "Creating font directory at: {}",
        &output_dir.to_string_lossy().cyan()
    );
    std::fs::create_dir_all(&files_download_dir)?;
    // let total_files = font_family.files.len();
    // let progress = Arc::new(Mutex::new(ProgressState {
    //     total: total_files,
    //     completed: 0,
    //     downloaded_files: Vec::with_capacity(total_files),
    // }));
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
        let task = tokio::spawn(async move {
            download_font_file(
                &client_clone,
                &download_url,
                family_name_clone,
                font_style,
                files_download_dir_clone,
            )
            .await
            .unwrap()
        });
        download_tasks.push(task);
    }
    // println!(
    //     "{}",
    //     format!(
    //         "Installed {} fonts in {}ms",
    //         // progress_state.completed,
    //         duration.as_millis()
    //     )
    //     .dimmed()
    // );
    while let Some(result) = download_tasks.next().await {
        result.unwrap();
    }
    Ok(())
}

async fn download_font_file(
    client: &Client,
    url: &str,
    family_name: String,
    font_style: FontStyles,
    files_download_dir: PathBuf,
) -> Result<(), String> {
    let output_path = files_download_dir.join(format!("{}-{}.ttf", family_name, &font_style));
    println!(
        " {} {}{}",
        "+".green(),
        &family_name,
        format!("=={}", &font_style).dimmed(),
    );
    let response = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from {}", &url)))?;
    let mut file = File::create(&output_path).or(Err(format!(
        "Failed to create file at: {}",
        &output_path.to_string_lossy()
    )))?;
    let bytes = response
        .bytes()
        .await
        .or(Err(format!("No response bytes from request url: {}", url)))?;
    file.write_all(&bytes).or(Err(format!(
        "Couldn't write response {:#?} to file {}",
        bytes,
        output_path.to_string_lossy()
    )))?;
    // TODO: This should also be its own function
    // TODO: ideally this should download and build the woff2_compress binary if it doesn't exist
    // and then run it on the files instead of shipping with it by default
    let mut process = Popen::create(
        &["./woff2_compress", &output_path.to_string_lossy()],
        PopenConfig {
            stdout: subprocess::Redirection::Pipe,
            ..Default::default()
        },
    )
    .unwrap();
    if let Some(_) = process.poll() {
    } else {
        let _ = process.terminate();
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
