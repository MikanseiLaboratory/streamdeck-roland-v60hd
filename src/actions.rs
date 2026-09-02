use roland_rs::devices::v60hd::{self, Command};
use roland_rs::RolandError;

use crate::settings::ActionSettings;

pub const SELECT_PGM: &str = "com.flowingspdg.roland.v60hd.select.pgm";
pub const SELECT_PST: &str = "com.flowingspdg.roland.v60hd.select.pst";
pub const SELECT_AUX: &str = "com.flowingspdg.roland.v60hd.select.aux";
pub const CUT: &str = "com.flowingspdg.roland.v60hd.cut";
pub const AUTO: &str = "com.flowingspdg.roland.v60hd.auto";
pub const COMPOSITION: &str = "com.flowingspdg.roland.v60hd.composition";
pub const TRANSITION: &str = "com.flowingspdg.roland.v60hd.transition";
pub const PINP: &str = "com.flowingspdg.roland.v60hd.pinp";
pub const SPLIT: &str = "com.flowingspdg.roland.v60hd.split";
pub const DSK: &str = "com.flowingspdg.roland.v60hd.dsk";
pub const OUTPUT_ASSIGN: &str = "com.flowingspdg.roland.v60hd.output.assign";
pub const CHANNEL6: &str = "com.flowingspdg.roland.v60hd.channel6";
pub const MEMORY: &str = "com.flowingspdg.roland.v60hd.memory";
pub const AUDIO: &str = "com.flowingspdg.roland.v60hd.audio";
pub const SYSTEM: &str = "com.flowingspdg.roland.v60hd.system";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    Down,
    Up,
}

#[derive(Debug)]
pub enum DeviceJob {
    Send(Command),
}

pub fn build_job(
    action: &str,
    settings: &ActionSettings,
    gesture: Gesture,
) -> Result<Option<DeviceJob>, String> {
    if gesture != Gesture::Down {
        return Ok(None);
    }
    match action {
        SELECT_PGM => Ok(Some(DeviceJob::Send(v60hd::pgm(parse_channel(
            &settings.source,
        )?)))),
        SELECT_PST => Ok(Some(DeviceJob::Send(v60hd::pst(parse_channel(
            &settings.source,
        )?)))),
        SELECT_AUX => Ok(Some(DeviceJob::Send(v60hd::aux(parse_channel(
            &settings.source,
        )?)))),
        CUT => Ok(Some(DeviceJob::Send(v60hd::cut()))),
        AUTO => Ok(Some(DeviceJob::Send(v60hd::auto()))),
        COMPOSITION => composition_job(settings),
        TRANSITION => transition_job(settings),
        PINP => pinp_job(settings),
        SPLIT => split_job(settings),
        DSK => dsk_job(settings),
        OUTPUT_ASSIGN => Ok(Some(DeviceJob::Send(output_command(settings)?))),
        CHANNEL6 => Ok(Some(DeviceJob::Send(v60hd::set_channel6_input(
            parse_channel6(&settings.channel6)?,
        )))),
        MEMORY => {
            let slot = v60hd::MemorySlot::new(parse_u8(&settings.slot, "slot")?).map_err(err)?;
            Ok(Some(DeviceJob::Send(v60hd::load_memory(slot))))
        }
        AUDIO => audio_job(settings),
        SYSTEM => system_job(settings),
        _ => Err(format!("unknown action {action}")),
    }
}

fn composition_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    Ok(Some(DeviceJob::Send(
        match settings.composition_op.as_str() {
            "pinp1" => v60hd::pinp1_sw(),
            "pinp2" => v60hd::pinp2_sw(),
            "split" => v60hd::split_sw(),
            "dsk" => v60hd::dsk_sw(),
            "dsk_pvw" => v60hd::dsk_pvw(),
            "auto_mixing" => v60hd::auto_mixing(),
            "output_fade" => v60hd::output_fade(),
            other => return Err(format!("unknown composition {other}")),
        },
    )))
}

fn transition_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    match settings.trans_op.as_str() {
        "type" => Ok(Some(DeviceJob::Send(v60hd::set_transition(
            parse_transition(&settings.trans_type)?,
        )))),
        "time" => {
            let time =
                v60hd::TransitionTime::new(parse_u8(&settings.tenths, "tenths")?).map_err(err)?;
            Ok(Some(DeviceJob::Send(v60hd::set_transition_time(time))))
        }
        other => Err(format!("unknown transition operation {other}")),
    }
}

fn pinp_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    let pos = v60hd::PinPPosition::new(
        parse_i16(&settings.pos_h, "horizontal")?,
        parse_i16(&settings.pos_v, "vertical")?,
    )
    .map_err(err)?;
    Ok(Some(DeviceJob::Send(match settings.pinp_key.as_str() {
        "1" | "pinp1" => v60hd::set_pinp1_position(pos),
        "2" | "pinp2" => v60hd::set_pinp2_position(pos),
        other => return Err(format!("invalid PinP {other}")),
    })))
}

fn split_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    let pos = v60hd::SplitPosition::new(
        parse_i16(&settings.pos_h, "position 1")?,
        parse_i16(&settings.pos_v, "position 2")?,
    )
    .map_err(err)?;
    Ok(Some(DeviceJob::Send(v60hd::set_split_position(pos))))
}

fn dsk_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    match settings.dsk_op.as_str() {
        "source" => Ok(Some(DeviceJob::Send(v60hd::set_dsk_source(parse_channel(
            &settings.source,
        )?)))),
        "key_level" => Ok(Some(DeviceJob::Send(v60hd::set_dsk_key_level(parse_u8(
            &settings.value,
            "level",
        )?)))),
        "key_gain" => Ok(Some(DeviceJob::Send(v60hd::set_dsk_key_gain(parse_u8(
            &settings.value,
            "gain",
        )?)))),
        other => Err(format!("unknown DSK operation {other}")),
    }
}

fn output_command(settings: &ActionSettings) -> Result<Command, String> {
    let bus = parse_output_bus(&settings.output_assign)?;
    Ok(match settings.output.as_str() {
        "sdi1" => v60hd::set_sdi1_bus(bus),
        "sdi2" => v60hd::set_sdi2_bus(bus),
        "hdmi1" => v60hd::set_hdmi1_bus(bus),
        "hdmi2" => v60hd::set_hdmi2_bus(bus),
        other => return Err(format!("invalid output {other}")),
    })
}

fn audio_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    match settings.audio_op.as_str() {
        "input_level" => {
            let level = parse_audio_level(&settings.value)?;
            Ok(Some(DeviceJob::Send(v60hd::set_input_audio_level(
                parse_audio_input(&settings.audio_input)?,
                level,
            ))))
        }
        "master" => Ok(Some(DeviceJob::Send(v60hd::set_master_level(
            parse_audio_level(&settings.value)?,
        )))),
        "aux" => Ok(Some(DeviceJob::Send(v60hd::set_aux_level(
            parse_audio_level(&settings.value)?,
        )))),
        "delay" => {
            let delay = v60hd::AudioDelay::new(parse_u8(&settings.value, "delay")?).map_err(err)?;
            Ok(Some(DeviceJob::Send(v60hd::set_input_audio_delay(
                parse_analog_input(&settings.analog_input)?,
                delay,
            ))))
        }
        "mute" => Ok(Some(DeviceJob::Send(v60hd::mute_input(parse_audio_input(
            &settings.audio_input,
        )?)))),
        "solo" => Ok(Some(DeviceJob::Send(v60hd::solo_input(parse_audio_input(
            &settings.audio_input,
        )?)))),
        other => Err(format!("unknown audio operation {other}")),
    }
}

fn system_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    Ok(Some(DeviceJob::Send(match settings.system_op.as_str() {
        "hdcp" => v60hd::set_hdcp(parse_hdcp(&settings.hdcp)?),
        "test_pattern" => v60hd::set_test_pattern(parse_test_pattern(&settings.test_pattern)?),
        "test_tone" => v60hd::set_test_tone(parse_test_tone(&settings.test_tone)?),
        other => return Err(format!("unknown system operation {other}")),
    })))
}

fn err(e: RolandError) -> String {
    e.to_string()
}

fn parse_u8(s: &str, name: &str) -> Result<u8, String> {
    s.trim().parse().map_err(|_| format!("invalid {name}: {s}"))
}

fn parse_i16(s: &str, name: &str) -> Result<i16, String> {
    s.trim().parse().map_err(|_| format!("invalid {name}: {s}"))
}

pub fn parse_channel(s: &str) -> Result<v60hd::Channel, String> {
    let s = s.trim();
    if let Ok(index) = s.parse::<u8>() {
        return v60hd::Channel::from_u8(index).map_err(err);
    }
    let (kind, n) = s
        .split_once(':')
        .ok_or_else(|| format!("invalid channel {s}"))?;
    let n: u8 = n.parse().map_err(|_| format!("invalid channel {s}"))?;
    match kind {
        "sdi" => v60hd::Channel::sdi(n).map_err(err),
        "hdmi" if n == 5 => Ok(v60hd::Channel::Hdmi5),
        "hdmi_rgb" | "hdmi-rgb" if n == 6 => Ok(v60hd::Channel::HdmiRgb6),
        "still" => v60hd::Channel::still(n).map_err(err),
        _ => Err(format!("invalid channel {s}")),
    }
}

fn parse_transition(s: &str) -> Result<v60hd::Transition, String> {
    Ok(match s {
        "mix" => v60hd::Transition::Mix,
        "wipe1" => v60hd::Transition::Wipe1,
        "wipe2" => v60hd::Transition::Wipe2,
        other => return Err(format!("invalid transition {other}")),
    })
}

fn parse_output_bus(s: &str) -> Result<v60hd::OutputBus, String> {
    Ok(match s {
        "pgm" | "program" => v60hd::OutputBus::Program,
        "pvw" | "preview" => v60hd::OutputBus::Preview,
        "aux" => v60hd::OutputBus::Aux,
        other => return Err(format!("invalid bus {other}")),
    })
}

fn parse_channel6(s: &str) -> Result<v60hd::Channel6Input, String> {
    Ok(match s {
        "hdmi" => v60hd::Channel6Input::Hdmi,
        "rgb" | "rgb_component" => v60hd::Channel6Input::RgbComponent,
        other => return Err(format!("invalid channel 6 input {other}")),
    })
}

fn parse_audio_input(s: &str) -> Result<v60hd::AudioInput, String> {
    v60hd::AudioInput::from_u8(parse_u8(s, "audio input")?).map_err(err)
}

fn parse_analog_input(s: &str) -> Result<v60hd::AnalogAudioInput, String> {
    v60hd::AnalogAudioInput::from_u8(parse_u8(s, "analog input")?).map_err(err)
}

fn parse_audio_level(s: &str) -> Result<v60hd::AudioLevel, String> {
    v60hd::AudioLevel::from_tenths(s.trim().parse().map_err(|_| format!("invalid level {s}"))?)
        .map_err(err)
}

fn parse_hdcp(s: &str) -> Result<v60hd::Hdcp, String> {
    Ok(match s {
        "off" => v60hd::Hdcp::Off,
        "on" => v60hd::Hdcp::On,
        other => return Err(format!("invalid HDCP {other}")),
    })
}

fn parse_test_pattern(s: &str) -> Result<v60hd::TestPattern, String> {
    v60hd::TestPattern::from_u8(parse_u8(s, "test pattern")?).map_err(err)
}

fn parse_test_tone(s: &str) -> Result<v60hd::TestTone, String> {
    v60hd::TestTone::from_u8(parse_u8(s, "test tone")?).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pgm_sdi1_encodes_stx_frame() {
        let settings = ActionSettings {
            source: "sdi:1".into(),
            ..ActionSettings::default()
        };
        let job = build_job(SELECT_PGM, &settings, Gesture::Down)
            .unwrap()
            .unwrap();
        match job {
            DeviceJob::Send(cmd) => assert_eq!(cmd.encode(), "\x02PGM:0;"),
        }
    }

    #[test]
    fn cut_is_stx_cut() {
        let job = build_job(CUT, &ActionSettings::default(), Gesture::Down)
            .unwrap()
            .unwrap();
        match job {
            DeviceJob::Send(cmd) => assert_eq!(cmd.encode(), "\x02CUT;"),
        }
        assert!(build_job(CUT, &ActionSettings::default(), Gesture::Up)
            .unwrap()
            .is_none());
    }

    #[test]
    fn still8_is_channel_7() {
        assert_eq!(parse_channel("still:8").unwrap(), v60hd::Channel::Still8);
        assert_eq!(parse_channel("hdmi:5").unwrap(), v60hd::Channel::Hdmi5);
    }
}
