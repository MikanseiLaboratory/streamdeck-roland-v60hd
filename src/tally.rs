use roland_rs::devices::v60hd::TallyColor;

use crate::actions::{SELECT_PGM, SELECT_PST};
use crate::settings::ActionSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TallyCheck {
    Off,
    Pgm,
    Prv,
    Both,
}

impl TallyCheck {
    pub fn parse(value: &str, action: &str) -> Self {
        match value {
            "off" => Self::Off,
            "pgm" => Self::Pgm,
            "prv" => Self::Prv,
            "both" => Self::Both,
            _ if action == SELECT_PGM => Self::Pgm,
            _ if action == SELECT_PST => Self::Prv,
            _ => Self::Off,
        }
    }

    pub fn light(self, color: TallyColor) -> Option<TallyLight> {
        match self {
            Self::Off => None,
            Self::Pgm if color.is_program() => Some(TallyLight::Program),
            Self::Pgm => None,
            Self::Prv if color.is_preview() => Some(TallyLight::Preview),
            Self::Prv => None,
            Self::Both => match color {
                TallyColor::Dark => None,
                TallyColor::Red => Some(TallyLight::Program),
                TallyColor::Green => Some(TallyLight::Preview),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TallyLight {
    Program,
    Preview,
    #[allow(dead_code)]
    Both,
}

#[derive(Debug, Clone)]
pub struct TallyBinding {
    pub source: Option<u8>,
    pub check: TallyCheck,
}

impl TallyBinding {
    pub fn from_action(action: &str, settings: &ActionSettings) -> Self {
        Self {
            source: tally_source_index(&settings.source),
            check: TallyCheck::parse(&settings.tally_check, action),
        }
    }

    pub fn watches_tally(&self) -> bool {
        self.check != TallyCheck::Off && self.source.is_some()
    }
}

pub fn tally_source_index(source: &str) -> Option<u8> {
    crate::actions::parse_channel(source)
        .ok()
        .map(|ch| ch.as_u8())
}

pub fn image_data_uri(light: TallyLight) -> String {
    let svg = match light {
        TallyLight::Program => solid("#E10600"),
        TallyLight::Preview => solid("#00A651"),
        TallyLight::Both => {
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"144\" height=\"144\"><rect width=\"144\" height=\"72\" fill=\"#E10600\"/><rect y=\"72\" width=\"144\" height=\"72\" fill=\"#00A651\"/></svg>"
                .to_string()
        }
    };
    format!("data:image/svg+xml;charset=utf8,{svg}")
}

fn solid(fill: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144"><rect width="144" height="144" fill="{fill}"/></svg>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_map_to_tally_index() {
        assert_eq!(tally_source_index("sdi:1"), Some(0));
        assert_eq!(tally_source_index("hdmi:5"), Some(4));
        assert_eq!(tally_source_index("still:8"), Some(7));
    }

    #[test]
    fn both_check_lights_red_and_green() {
        let check = TallyCheck::Both;
        assert_eq!(check.light(TallyColor::Red), Some(TallyLight::Program));
        assert_eq!(check.light(TallyColor::Green), Some(TallyLight::Preview));
        assert_eq!(check.light(TallyColor::Dark), None);
    }

    #[test]
    fn off_never_lights() {
        assert_eq!(TallyCheck::Off.light(TallyColor::Red), None);
        assert_eq!(TallyCheck::Off.light(TallyColor::Green), None);
    }
}
