use std::fmt::Display;

#[derive(Clone)]
pub enum Extension {
    /// Extension not supported by pulseq-rs. Raw data is stored.
    Unsupported { string_id: String, data: String },
    /// Set the value of one of the supported pulseq flags
    LabelSet { flag: ExtLabelFlag, value: i32 },
    /// Increase the value of one of the counter flags
    LabelInc {
        counter: ExtLabelCounter,
        value: i32,
    },
    /// Siemens specific extension for triggering on external channels.
    /// `typ` and `channel` are raw vendor-specific numbers; `delay` and
    /// `duration` are stored in seconds (parsed from microseconds in the file).
    Trigger {
        typ: u32,
        channel: u32,
        delay: f64,
        duration: f64,
    },
}

impl Extension {
    pub fn parse(string_id: &str, data: &str) -> Self {
        match string_id.to_lowercase().as_str() {
            "labelset" => parse_labelset(data),
            "labelinc" => parse_labelinc(data),
            "triggers" => parse_trigger(data),
            _ => Self::Unsupported {
                string_id: string_id.to_owned(),
                data: data.to_owned(),
            },
        }
    }
}

// TODO: should return parsing errors instead of panicking

fn parse_label_inner(data: &str) -> (i32, ExtLabelFlag) {
    let (value, flag) = data.split_once(' ').unwrap();
    let value: i32 = value.trim().parse().unwrap();

    let flag = match flag.trim().to_uppercase().as_str() {
        "SLC" => ExtLabelFlag::Counter(ExtLabelCounter::Slc),
        "SEG" => ExtLabelFlag::Counter(ExtLabelCounter::Seg),
        "REP" => ExtLabelFlag::Counter(ExtLabelCounter::Rep),
        "AVG" => ExtLabelFlag::Counter(ExtLabelCounter::Avg),
        "SET" => ExtLabelFlag::Counter(ExtLabelCounter::Set),
        "ECO" => ExtLabelFlag::Counter(ExtLabelCounter::Eco),
        "PHS" => ExtLabelFlag::Counter(ExtLabelCounter::Phs),
        "LIN" => ExtLabelFlag::Counter(ExtLabelCounter::Lin),
        "PAR" => ExtLabelFlag::Counter(ExtLabelCounter::Par),
        "ACQ" => ExtLabelFlag::Counter(ExtLabelCounter::Acq),
        "NAV" => ExtLabelFlag::Nav,
        "REV" => ExtLabelFlag::Rev,
        "SMS" => ExtLabelFlag::Sms,
        "REF" => ExtLabelFlag::Ref,
        "IMA" => ExtLabelFlag::Ima,
        "NOISE" => ExtLabelFlag::Noise,
        "PMC" => ExtLabelFlag::Pmc,
        "NOROT" => ExtLabelFlag::NoRot,
        "NOPOS" => ExtLabelFlag::NoPos,
        "NOSCL" => ExtLabelFlag::NoScl,
        "ONCE" => ExtLabelFlag::Once,
        "TRID" => ExtLabelFlag::Counter(ExtLabelCounter::Trid),
        _ => panic!("Unsupported flag '{flag}'"),
    };

    (value, flag)
}

fn parse_labelset(data: &str) -> Extension {
    let (value, flag) = parse_label_inner(data);
    Extension::LabelSet { flag, value }
}

fn parse_labelinc(data: &str) -> Extension {
    let (value, flag) = parse_label_inner(data);
    let counter = match flag {
        ExtLabelFlag::Counter(counter) => counter,
        flag => panic!("Cannot use 'LABELINC' on a flag ({flag:?}), only on counters"),
    };

    Extension::LabelInc { counter, value }
}

fn parse_trigger(data: &str) -> Extension {
    let parts: [&str; 4] = data
        .split_whitespace()
        .collect::<Vec<_>>()
        .try_into()
        .expect("trigger extension expects 4 numbers");

    Extension::Trigger {
        typ: parts[0].parse().unwrap(),
        channel: parts[1].parse().unwrap(),
        delay: parts[2].parse::<f64>().unwrap() * 1e-6,
        duration: parts[3].parse::<f64>().unwrap() * 1e-6,
    }
}

/// Labels and their descriptions taken from pypulseq - unknown labels throw an error.
/// Flags or counters - can be set but not necessarily increased.
/// https://github.com/imr-framework/pypulseq/blob/master/src/pypulseq/make_label.py
#[derive(Debug, Clone, Copy)]
pub enum ExtLabelFlag {
    /// navigator data flag.
    Nav,
    /// flag indicating that the readout direction is reversed.
    Rev,
    /// simultaneous multi-slice (SMS) acquisition.
    Sms,
    /// parallel imaging flag indicating reference / auto-calibration data.
    Ref,
    /// parallel imaging flag indicating imaging data within the ACS region.
    Ima,
    /// noise adjust scan, for iPAT acceleration.
    Noise,
    /// for MoCo/PMC Pulseq version to recognize blocks that can be prospectively corrected for motion.
    Pmc,
    /// instruct the interpreter to ignore the rotation of the FOV specified on the UI.
    NoRot,
    /// instruct the interpreter to ignore the position of the FOV specified on the UI.
    NoPos,
    /// instruct the interpreter to ignore the scaling of the FOV specified on the UI.
    NoScl,
    /// a 3-state flag that instructs the interpreter as follows:
    /// - `ONCE == 0`: blocks are executed on every repetition;
    /// - `ONCE == 1`: only the first repetition of the block is executed;
    /// - `ONCE == 2`: only the last repetition of the block is executed.
    Once,
    /// Counters are also flags (can be set)
    Counter(ExtLabelCounter),
}

/// Counters - can be used with labelset and labelinc. Own type to group the inc-able flags.
#[derive(Debug, Clone, Copy)]
pub enum ExtLabelCounter {
    /// slice counter (or slab counter for 3D multi-slab sequences).
    Slc,
    /// segment counter e.g. for segmented FLASH or EPI.
    Seg,
    /// repetition counter.
    Rep,
    /// averaging counter.
    Avg,
    /// flexible counter without firm assignment.
    Set,
    /// echo counter in multi-echo sequences.
    Eco,
    /// cardiac phase counter.
    Phs,
    /// line counter in 2D and 3D acquisitions.
    Lin,
    /// partition counter; itt counts phase encoding steps in the 2nd (through-slab) phase encoding direction in 3D sequences.
    Par,
    /// spectroscopic acquisition counter.
    Acq,
    /// marks the beginning of a repeatable module in the sequence (e.g. TR).
    Trid,
}

impl Display for ExtLabelFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtLabelFlag::Nav => f.write_str("NAV"),
            ExtLabelFlag::Rev => f.write_str("REV"),
            ExtLabelFlag::Sms => f.write_str("SMS"),
            ExtLabelFlag::Ref => f.write_str("REF"),
            ExtLabelFlag::Ima => f.write_str("IMA"),
            ExtLabelFlag::Noise => f.write_str("NOISE"),
            ExtLabelFlag::Pmc => f.write_str("PMC"),
            ExtLabelFlag::NoRot => f.write_str("NOROT"),
            ExtLabelFlag::NoPos => f.write_str("NOPOS"),
            ExtLabelFlag::NoScl => f.write_str("NOSCL"),
            ExtLabelFlag::Once => f.write_str("ONCE"),
            ExtLabelFlag::Counter(counter) => counter.fmt(f),
        }
    }
}

impl Display for ExtLabelCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtLabelCounter::Slc => f.write_str("SLC"),
            ExtLabelCounter::Seg => f.write_str("SEG"),
            ExtLabelCounter::Rep => f.write_str("REP"),
            ExtLabelCounter::Avg => f.write_str("AVG"),
            ExtLabelCounter::Set => f.write_str("SET"),
            ExtLabelCounter::Eco => f.write_str("ECO"),
            ExtLabelCounter::Phs => f.write_str("PHS"),
            ExtLabelCounter::Lin => f.write_str("LIN"),
            ExtLabelCounter::Par => f.write_str("PAR"),
            ExtLabelCounter::Acq => f.write_str("ACQ"),
            ExtLabelCounter::Trid => f.write_str("TRID"),
        }
    }
}
