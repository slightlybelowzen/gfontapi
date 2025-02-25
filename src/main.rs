use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    process,
};

const BASE_URL: &str = "https://www.googleapis.com/webfonts/v1/webfonts";

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

#[derive(Parser)]
#[command(name = "gfontapi")]
#[command(version = "0.1.0")]
#[command(about = "manage google webfonts for your application")]
#[command(
    help_template = "{name} v{version}\n\n{about}\n\nusage: {name} [options] [fontname]\n\noptions\n{options}"
)]
struct Args {
    /// Name of the font to download
    #[arg(value_name = "fontname")]
    fontname: String,
    /// Directory to place the converted fonts
    #[arg(
        short,
        long = "target-dir",
        help_heading = "Options",
        name = "path",
        help = "target directory, defaults to ./fonts."
    )]
    target_dir: Option<PathBuf>,
    /// Users google application API key
    #[arg(
        short,
        long = "api-key",
        help_heading = "Options",
        name = "key",
        help = "google api key generated from developer console, can also be set as `EXPORT GFONT_API_KEY=<API_KEY>`"
    )]
    api_key: Option<String>,
}

fn get_api_key(cli_api_key: Option<String>) -> String {
    let api_key;
    // Get api_key if passed to cli, or try from environment variable
    if let Some(key) = cli_api_key {
        api_key = key;
    } else if let Ok(env_key) = env::var("GFONT_API_KEY") {
        api_key = env_key;
    } else {
        eprintln!("\x1b[91merror\x1b[0m: Using `gfontapi` requires an API key. Pass it from either the command line using `gfontapi --api-key=YOUR_API_KEY` or an environment variable `exp
ort GFONT_API_KEY=YOUR_API_KEY`");
        process::exit(1);
    }
    api_key
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let output_dir: PathBuf;
    if let Some(path) = args.target_dir {
        output_dir = path
    } else {
        output_dir = PathBuf::from("./fonts")
    };
    println!("output_dir: {:#?}", output_dir);
    let api_key = get_api_key(args.api_key);
    let api_url = format!(
        "{base_url}?key={key}&family={fontname}",
        base_url = BASE_URL,
        key = api_key,
        fontname = args.fontname
    );
    let client = reqwest::Client::new();

    let response = client.get(api_url).send().await?;
    let body = response.text().await?;
    let val: Font = serde_json::from_str(&body)?;
    println!("{:#?}", val);

    let font_family = &val.items[0];
    let mut download_tasks = Vec::new();
    let family_name = &font_family.family;
    let files_download_dir = output_dir.join(family_name);
    println!("files_download_dir: {:#?}", files_download_dir);
    if !Path::new(&files_download_dir).exists() {
        std::fs::create_dir(&files_download_dir)?;
    }
    for (variant, url) in &font_family.files {
        let variant_name = variant.clone();
        let download_url = url.clone();
        let output_path = files_download_dir.join(variant_name);
        println!("output_path: {:#?}", files_download_dir);
        let task = tokio::spawn(async move { download_font_file(&download_url, &output_path) });
        download_tasks.push(task);
    }
    // let results = tokio::join!(download_tasks).await;
    Ok(())
}

fn download_font_file(
    url: &str,
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("downloading {} to {:#?}", url, output_path);
    Ok(())
}
