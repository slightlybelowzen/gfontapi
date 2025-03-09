use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::Display;

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
