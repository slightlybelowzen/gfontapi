use std::{env, fs::OpenOptions, path::PathBuf, process};

use owo_colors::OwoColorize;
use std::io::Write;
use subprocess::{Popen, PopenConfig, Redirection};

use crate::FontStyles;

/// Gets the path to the `woff2_compress` binary.
/// Looks for `woff2_compress` in `~/.gfontapi/bin` and `/usr/local/bin` if not found, returns an error
pub fn get_woff2_compress() -> Result<PathBuf, String> {
    let binary_exists: Vec<PathBuf> = [
        "/usr/local/bin/woff2_compress",
        "~/.gfontapi/bin/woff2_compress",
    ]
    .iter()
    .map(|x| PathBuf::from(x))
    .filter(|x| x.exists())
    .collect();
    if binary_exists.len() == 0 {
        return Err(format!("Could not locate woff2_compress binary on system"));
    }
    Ok(binary_exists[0].clone())
}

/// Convert the font name to kebab case
pub fn format_font_string(input: &str) -> String {
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

/// Writes a css file for a font family to the font directory.
/// Creates an `@font-face` rule for each font style in the downloaded fonts
pub(crate) fn write_css_file_for_font(
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

/// Converts a ttf font file to a woff2 font file using the `woff2_compress` tool.
/// Uses the `get_woff2_compress` function to get the path to the `woff2_compress` binary or returns an error
pub fn convert_to_woff2(ttf_path: &PathBuf) -> Result<(), String> {
    let woff2_compress = match get_woff2_compress() {
        Ok(path) => path,
        Err(e) => return Err(e),
    };
    let mut process = Popen::create(
        &[woff2_compress, ttf_path.clone()],
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

/// Gets the API key from the environment variable `GFONT_API_KEY` or the CLI argument `--api-key`
pub fn get_api_key(cli_api_key: Option<String>) -> String {
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

pub fn get_output_dir(target_dir: Option<PathBuf>) -> PathBuf {
    target_dir.unwrap_or_else(|| PathBuf::from("./fonts"))
}
