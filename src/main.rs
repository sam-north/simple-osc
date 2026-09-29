use nih_plug::prelude::*;
use simple_osc::SimpleOsc;

mod standalone_window;

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    // `--no-midi` skips connecting a MIDI keyboard, e.g. to test with only the computer keys.
    let no_midi = args.iter().any(|a| a == "--no-midi");
    args.retain(|a| a != "--no-midi");

    // The standalone wrapper doesn't connect MIDI unless told to. If no `--midi-input` was given,
    // connect the first available MIDI input so a plugged-in keyboard just works.
    if !no_midi && !args.iter().any(|a| a.starts_with("--midi-input")) {
        if let Some(port) = first_midi_input() {
            println!("Using MIDI input: {port}");
            args.push("--midi-input".into());
            args.push(port);
        } else {
            println!("No MIDI input found, use the computer keyboard (A-L and W-P).");
        }
    }
    // The wrapper treats the period size as a hard maximum, but Windows (WASAPI shared mode) picks
    // its own callback size, e.g. 1056 samples, and the wrapper panics if it gets more than it
    // asked for. Ask for plenty of headroom; actual latency is still decided by Windows.
    if !args
        .iter()
        .any(|a| a == "-p" || a.starts_with("--period-size"))
    {
        args.push("--period-size".into());
        args.push("2048".into());
    }

    standalone_window::fix_up_when_open(SimpleOsc::NAME);
    nih_export_standalone_with_args::<SimpleOsc, _>(args);
}

fn first_midi_input() -> Option<String> {
    let midi_in = midir::MidiInput::new("simple-osc-probe").ok()?;
    let port = midi_in.ports().into_iter().next()?;
    midi_in.port_name(&port).ok()
}
