use owo_colors::OwoColorize;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process};
use strum::Display;

use crate::BASE_URL;

#[derive(Display, Clone, Debug)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum FontStyles {
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
    pub(crate) fn get_style_and_weight(&self) -> (&'static str, u16) {
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
pub struct Font {
    pub(crate) items: Vec<FontFamily>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct FontFamily {
    pub family: String,
    pub variants: Vec<String>,
    pub subsets: Vec<String>,
    pub files: HashMap<String, String>,
    pub category: String,
}

pub(crate) fn transpile_font_weight(font_string: &str) -> Result<FontStyles, String> {
    let font_weight_mappings: HashMap<&'static str, FontStyles> = HashMap::from([
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

pub(crate) async fn fetch_font_data(
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

    let response = client
        .get(&api_url)
        .send()
        .await
        .map_err(|err| {
            eprintln!(
                "{}: Failed to fetch `{}`\n  {}: {}",
                "error".red(),
                &api_url,
                "Caused by".red(),
                err
            );
            process::exit(1);
        })
        .unwrap();

    if response.status() != StatusCode::OK {
        let status = response.status();
        eprintln!(
            "{}: Failed to fetch `{}`\n  {}: {}",
            "error".red(),
            &api_url,
            "Caused by".red(),
            status
        );
        process::exit(1);
    }

    let body = response.text().await?;
    let font_data: Font = serde_json::from_str(&body)
        .map_err(|_| eprintln!("Could not parse response"))
        .unwrap();

    Ok(font_data.items[0].clone())
}
