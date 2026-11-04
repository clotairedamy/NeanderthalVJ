//! MIDI control: any controller plugged in can play the visuals.
//!
//! Every input port is opened. Notes switch modes and start and stop the
//! track; the first eight continuous controllers ride the controls a VJ
//! reaches for most. The mapping is fixed and printed in the Source tab,
//! so a controller works the moment it is plugged in, with nothing to set
//! up.

use std::sync::mpsc::{channel, Receiver};

use midir::{MidiInput, MidiInputConnection};

use crate::state::{Mode, Settings, MODES};

/// A message from a controller, as the app uses it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MidiEvent {
    Note(u8),
    Control(u8, u8),
}

/// What a message asks the app to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MidiAction {
    Mode(Mode),
    PlayPause,
    Performance,
    /// A setting changed; already applied to the settings passed in.
    Set,
    Nothing,
}

/// The mapping, as the Source tab shows it: control, then what it does.
pub const MAPPING: [(&str, &str); 11] = [
    ("Notes C, C#, D, D#", "Visualizations 1 to 4, in any octave"),
    ("Note E", "Play or pause the track"),
    ("Note F", "Performance: hide or show the controls"),
    ("CC 1 (mod wheel)", "Sensitivity"),
    ("CC 2", "Reactivity of the head"),
    ("CC 3", "Beat impulse"),
    ("CC 4", "Bloom"),
    ("CC 5", "Beat flash"),
    ("CC 6", "Ghost echo"),
    ("CC 7", "The mode's own main control: rings' trails, the wall's waves, the head's point size"),
    ("CC 8", "Colour split on kicks"),
];

/// Apply one message to `s`, and say what else it asks for.
pub fn apply(event: MidiEvent, s: &mut Settings) -> MidiAction {
    match event {
        MidiEvent::Note(n) => match n % 12 {
            i @ 0..=3 => MidiAction::Mode(MODES[i as usize]),
            4 => MidiAction::PlayPause,
            5 => MidiAction::Performance,
            _ => MidiAction::Nothing,
        },
        MidiEvent::Control(cc, v) => {
            let x = v.min(127) as f64 / 127.0;
            match cc {
                1 => s.sensitivity = 0.1 + 2.9 * x,
                2 => s.pc_reactivity = 2.0 * x,
                3 => s.beat_impulse = 3.0 * x,
                4 => s.fx_bloom = 2.0 * x,
                5 => s.fx_flash = 2.0 * x,
                6 => s.fx_ghost = x,
                7 => match s.viz_mode {
                    Mode::Generative => s.gen_trails = x,
                    Mode::HidingGrid => s.grid_waves = 2.0 * x,
                    Mode::PointCloud | Mode::SynthCloud => s.pc_point_size = 0.5 + 5.5 * x,
                },
                8 => s.fx_chroma = 2.0 * x,
                _ => return MidiAction::Nothing,
            }
            MidiAction::Set
        }
    }
}

/// Decode a raw message: note-ons with a velocity, and control changes.
pub fn decode(bytes: &[u8]) -> Option<MidiEvent> {
    let (&status, rest) = bytes.split_first()?;
    match (status & 0xF0, rest) {
        (0x90, [note, vel, ..]) if *vel > 0 => Some(MidiEvent::Note(*note)),
        (0xB0, [cc, value, ..]) => Some(MidiEvent::Control(*cc, *value)),
        _ => None,
    }
}

/// The open ports and the messages arriving from them.
pub struct Midi {
    _connections: Vec<MidiInputConnection<()>>,
    pub devices: Vec<String>,
    pub events: Receiver<MidiEvent>,
}

/// Open every MIDI input on the machine.
pub fn open_all() -> Midi {
    let (tx, events) = channel();
    let mut connections = Vec::new();
    let mut devices = Vec::new();
    let Ok(probe) = MidiInput::new("NeanderthalVJ") else {
        return Midi { _connections: connections, devices, events };
    };
    for port in probe.ports() {
        let name = probe.port_name(&port).unwrap_or_else(|_| "MIDI input".into());
        // each connection needs an input of its own
        let Ok(input) = MidiInput::new("NeanderthalVJ") else { continue };
        let tx = tx.clone();
        let opened = input.connect(
            &port,
            "neanderthalvj-in",
            move |_, bytes, _| {
                if let Some(e) = decode(bytes) {
                    let _ = tx.send(e);
                }
            },
            (),
        );
        if let Ok(c) = opened {
            connections.push(c);
            devices.push(name);
        }
    }
    Midi { _connections: connections, devices, events }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_pick_modes_in_any_octave() {
        let mut s = Settings::default();
        assert_eq!(apply(MidiEvent::Note(60), &mut s), MidiAction::Mode(MODES[0]));
        assert_eq!(apply(MidiEvent::Note(63 + 12), &mut s), MidiAction::Mode(MODES[3]));
        assert_eq!(apply(MidiEvent::Note(64), &mut s), MidiAction::PlayPause);
        assert_eq!(apply(MidiEvent::Note(65), &mut s), MidiAction::Performance);
        assert_eq!(apply(MidiEvent::Note(67), &mut s), MidiAction::Nothing);
    }

    #[test]
    fn controllers_ride_the_controls_across_their_range() {
        let mut s = Settings::default();
        apply(MidiEvent::Control(4, 127), &mut s);
        assert!((s.fx_bloom - 2.0).abs() < 1e-9);
        apply(MidiEvent::Control(4, 0), &mut s);
        assert_eq!(s.fx_bloom, 0.0);
        s.viz_mode = Mode::HidingGrid;
        apply(MidiEvent::Control(7, 127), &mut s);
        assert!((s.grid_waves - 2.0).abs() < 1e-9, "CC 7 is the wall's waves in the grid");
    }

    #[test]
    fn raw_messages_decode_on_any_channel() {
        assert_eq!(decode(&[0x93, 60, 100]), Some(MidiEvent::Note(60)));
        // a note-on with no velocity is a note-off
        assert_eq!(decode(&[0x90, 60, 0]), None);
        assert_eq!(decode(&[0xB5, 7, 64]), Some(MidiEvent::Control(7, 64)));
        assert_eq!(decode(&[0x80, 60, 0]), None);
        assert_eq!(decode(&[]), None);
    }
}
