use crate::actions::{parse_channel, COMPOSITION, SELECT_AUX, SELECT_PGM, SELECT_PST};
use crate::settings::ActionSettings;
use roland_rs::devices::v60hd::{Channel, Composition, PanelStatus, TallyColor};

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
pub enum LightBinding {
    None,
    Tally {
        source: Option<u8>,
        check: TallyCheck,
    },
    Aux {
        channel: Option<Channel>,
    },
    Composition {
        op: String,
    },
}

impl LightBinding {
    pub fn from_action(action: &str, settings: &ActionSettings) -> Self {
        if action == SELECT_PGM || action == SELECT_PST {
            return Self::Tally {
                source: tally_source_index(&settings.source),
                check: TallyCheck::parse(&settings.tally_check, action),
            };
        }
        if action == SELECT_AUX {
            return Self::Aux {
                channel: parse_channel(&settings.source).ok(),
            };
        }
        if action == COMPOSITION {
            return Self::Composition {
                op: settings.composition_op.clone(),
            };
        }
        Self::None
    }

    pub fn light(
        &self,
        tally: Option<&[TallyColor; 8]>,
        panel: Option<&PanelStatus>,
    ) -> Option<TallyLight> {
        match self {
            Self::None => None,
            Self::Tally { source, check } => {
                let index = (*source)?;
                let color = tally.and_then(|states| states.get(index as usize).copied())?;
                check.light(color)
            }
            Self::Aux { channel } => {
                let channel = (*channel)?;
                let panel = panel?;
                if panel.aux == channel {
                    Some(TallyLight::Program)
                } else {
                    None
                }
            }
            Self::Composition { op } => {
                let panel = panel?;
                let on = match op.as_str() {
                    "pinp1" => panel.composition == Composition::PinP1,
                    "pinp2" => panel.composition == Composition::PinP2,
                    "split" => panel.composition == Composition::Split,
                    "dsk" => panel.dsk,
                    "output_fade" => panel.output_fade,
                    _ => false,
                };
                if on {
                    Some(TallyLight::Program)
                } else {
                    None
                }
            }
        }
    }
}

pub fn tally_source_index(source: &str) -> Option<u8> {
    parse_channel(source).ok().map(|ch| ch.as_u8())
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

    fn sample_panel() -> PanelStatus {
        PanelStatus {
            pgm: Channel::Sdi1,
            pst: Channel::Sdi2,
            aux: Channel::Sdi3,
            composition: Composition::PinP1,
            dsk: true,
            output_fade: false,
            video_fade_level: None,
        }
    }

    #[test]
    fn aux_lights_when_qpl_matches() {
        let settings = ActionSettings {
            source: "sdi:3".into(),
            ..ActionSettings::default()
        };
        let binding = LightBinding::from_action(SELECT_AUX, &settings);
        let panel = sample_panel();
        assert_eq!(binding.light(None, Some(&panel)), Some(TallyLight::Program));
        let other = ActionSettings {
            source: "sdi:1".into(),
            ..ActionSettings::default()
        };
        let other = LightBinding::from_action(SELECT_AUX, &other);
        assert_eq!(other.light(None, Some(&panel)), None);
    }

    #[test]
    fn composition_uses_qpl_fields() {
        let panel = sample_panel();
        let pinp1 = ActionSettings {
            composition_op: "pinp1".into(),
            ..ActionSettings::default()
        };
        let dsk = ActionSettings {
            composition_op: "dsk".into(),
            ..ActionSettings::default()
        };
        let fade = ActionSettings {
            composition_op: "output_fade".into(),
            ..ActionSettings::default()
        };
        let pvw = ActionSettings {
            composition_op: "dsk_pvw".into(),
            ..ActionSettings::default()
        };
        assert_eq!(
            LightBinding::from_action(COMPOSITION, &pinp1).light(None, Some(&panel)),
            Some(TallyLight::Program)
        );
        assert_eq!(
            LightBinding::from_action(COMPOSITION, &dsk).light(None, Some(&panel)),
            Some(TallyLight::Program)
        );
        assert_eq!(
            LightBinding::from_action(COMPOSITION, &fade).light(None, Some(&panel)),
            None
        );
        assert_eq!(
            LightBinding::from_action(COMPOSITION, &pvw).light(None, Some(&panel)),
            None
        );
    }
}
