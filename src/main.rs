use clap::Parser;
use std::{env, path::PathBuf, process};

#[derive(Parser)]
#[command(name = "gfontapi")]
#[command(version = "0.1.0")]
#[command(about = "manage google webfonts for your application")]
#[command(
    help_template = "{name} v{version}\n\n{about}\n\nusage: {name} [options] [fontname]\n\noptions\n{options}"
)]
struct Args {
    /// Name of the font to download
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
        help = "api key generated from google developer console. Can also be set as `EXPORT GFONT_API_KEY=<API_KEY>`"
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
        eprintln!("\x1b[91merror\x1b[0m: Using `gfontapi` requires an API key. Pass it from either the command line using `gfontapi --api-key=YOUR_API_KEY` or an environment variable `export GFONT_API_KEY=YOUR_API_KEY`");
        process::exit(1);
    }
    api_key
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // gfontapi --name=NAME --target-dir=TARGET_DIR --api-key=API_KEY
    let args = Args::parse();
    let _output_dir = match args.target_dir {
        None => PathBuf::from("./fonts"),
        Some(path) => path,
    };
    let api_key = get_api_key(args.api_key);
    let base_url = format!(
        "https://www.googleapis.com/webfonts/v1/webfonts?key={key}&family={fontname}",
        key = api_key,
        fontname = args.fontname
    );
    let client = reqwest::Client::new();
    let response = client.get(base_url).send().await?;
    let body = response.text().await?;
    println!("{}", body);
    Ok(())
}
