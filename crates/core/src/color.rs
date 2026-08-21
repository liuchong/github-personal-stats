use crate::{GithubStatsError, Theme};

pub const HEAT_RAMP_STEPS: usize = 4;

type Rgb = [u8; 3];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NamedRamp {
    name: &'static str,
    stops: [Rgb; HEAT_RAMP_STEPS],
}

const HEAT_ORANGE: NamedRamp = NamedRamp {
    name: "heat-orange",
    stops: [
        rgb("#ffe3ad"),
        rgb("#ffc65c"),
        rgb("#ffa726"),
        rgb("#fb8c00"),
    ],
};

const NAMED_RAMPS: [&NamedRamp; 6] = [
    &HEAT_ORANGE,
    &NamedRamp {
        name: "github-blue",
        stops: [
            rgb("#cfe0fa"),
            rgb("#8fbdf2"),
            rgb("#3b82d6"),
            rgb("#0b69d4"),
        ],
    },
    &NamedRamp {
        name: "forest",
        stops: [
            rgb("#d3ecd5"),
            rgb("#9fd6a6"),
            rgb("#4caf67"),
            rgb("#2e7d32"),
        ],
    },
    &NamedRamp {
        name: "violet",
        stops: [
            rgb("#e5dbf7"),
            rgb("#c0a7ee"),
            rgb("#8a63d2"),
            rgb("#6532c4"),
        ],
    },
    &NamedRamp {
        name: "crimson",
        stops: [
            rgb("#fbd7db"),
            rgb("#f3a3ac"),
            rgb("#e0576c"),
            rgb("#c62443"),
        ],
    },
    &NamedRamp {
        name: "graphite",
        stops: [
            rgb("#e4e8ee"),
            rgb("#c2c9d3"),
            rgb("#8b95a3"),
            rgb("#5b6572"),
        ],
    },
];

pub fn named_ramps() -> impl Iterator<Item = &'static str> {
    NAMED_RAMPS.iter().map(|ramp| ramp.name)
}

/// A ramp is opaque so a caller cannot describe one that does not resolve. The
/// only ways in are `parse` and `default`, and both produce stops for any theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeatRamp(Ramp);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ramp {
    Named(&'static NamedRamp),
    Seed(Rgb),
    Explicit([Rgb; HEAT_RAMP_STEPS]),
}

impl Default for HeatRamp {
    fn default() -> Self {
        Self(Ramp::Named(&HEAT_ORANGE))
    }
}

impl HeatRamp {
    pub fn parse(value: &str) -> Result<Self, GithubStatsError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(invalid("a colour name, one hex value, or four hex values"));
        }

        if let Some(named) = NAMED_RAMPS
            .iter()
            .find(|ramp| ramp.name.eq_ignore_ascii_case(trimmed))
        {
            return Ok(Self(Ramp::Named(named)));
        }

        let stops = trimmed
            .split(',')
            .map(str::trim)
            .filter(|stop| !stop.is_empty())
            .map(parse_hex)
            .collect::<Result<Vec<_>, _>>()?;

        match stops.len() {
            1 => Ok(Self(Ramp::Seed(stops[0]))),
            HEAT_RAMP_STEPS => Ok(Self(Ramp::Explicit([
                stops[0], stops[1], stops[2], stops[3],
            ]))),
            _ => Err(invalid(
                "expected a colour name, one hex value, or four comma separated hex values",
            )),
        }
    }

    pub fn stops(&self, theme: Theme) -> Vec<String> {
        match self.0 {
            Ramp::Explicit(stops) => stops.iter().copied().map(to_hex).collect(),
            Ramp::Named(named) => match theme {
                Theme::Dark => derive_dark(named.stops[HEAT_RAMP_STEPS - 1]),
                _ => named.stops.iter().copied().map(to_hex).collect(),
            },
            Ramp::Seed(seed) => match theme {
                Theme::Dark => derive_dark(seed),
                _ => derive_light(seed),
            },
        }
    }
}

/// Walks from a light tint down to the seed so the quiet end recedes into a
/// light background. Interpolating towards white in sRGB drains the hue out of
/// the middle stops, so the walk happens in OkLab.
fn derive_light(seed: Rgb) -> Vec<String> {
    let (lightness, a, b) = to_oklab(unit(seed));
    let chroma = a.hypot(b);
    let hue = b.atan2(a);
    let quietest = lightness.max(0.55) + (1.0 - lightness.max(0.55)) * 0.82;

    ramp_between(quietest, lightness, chroma, 0.28, hue)
}

/// The mirror of `derive_light`. On a dark card the quiet end has to sink
/// towards the background and the busy end has to climb, otherwise a one-commit
/// day outshines a fifty-commit day and the ring reads inside out. The quiet end
/// stops just above the dark ring track so a barely-active day stays
/// distinguishable from a gap.
const DARK_QUIET_LIGHTNESS: f64 = 0.32;
const DARK_BUSY_LIGHTNESS: f64 = 0.82;

fn derive_dark(seed: Rgb) -> Vec<String> {
    let (lightness, a, b) = to_oklab(unit(seed));
    let chroma = a.hypot(b);
    let hue = b.atan2(a);

    ramp_between(
        DARK_QUIET_LIGHTNESS,
        lightness.max(DARK_BUSY_LIGHTNESS),
        chroma,
        0.34,
        hue,
    )
}

fn ramp_between(
    quiet_lightness: f64,
    busy_lightness: f64,
    chroma: f64,
    quiet_chroma_share: f64,
    hue: f64,
) -> Vec<String> {
    (0..HEAT_RAMP_STEPS)
        .map(|index| {
            let progress = index as f64 / (HEAT_RAMP_STEPS - 1) as f64;
            let step_lightness = quiet_lightness + (busy_lightness - quiet_lightness) * progress;
            let step_chroma = chroma * (quiet_chroma_share + (1.0 - quiet_chroma_share) * progress);
            to_hex(quantize(from_oklab(
                step_lightness,
                step_chroma * hue.cos(),
                step_chroma * hue.sin(),
            )))
        })
        .collect()
}

fn invalid(message: &str) -> GithubStatsError {
    GithubStatsError::InvalidConfig {
        field: "heat_color",
        message: message.to_owned(),
    }
}

fn parse_hex(value: &str) -> Result<Rgb, GithubStatsError> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if digits.len() != 6 || !digits.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(invalid("hex values must look like #rrggbb"));
    }

    let channel = |start: usize| {
        u8::from_str_radix(&digits[start..start + 2], 16)
            .map_err(|_| invalid("hex values must look like #rrggbb"))
    };

    Ok([channel(0)?, channel(2)?, channel(4)?])
}

/// The palette table is written as hex so it stays readable, and parsed at
/// compile time so a typo in a stop is a build failure rather than a colour
/// nobody notices.
const fn rgb(hex: &str) -> Rgb {
    let digits = hex.as_bytes();
    [byte_at(digits, 1), byte_at(digits, 3), byte_at(digits, 5)]
}

const fn byte_at(digits: &[u8], index: usize) -> u8 {
    nibble(digits[index]) * 16 + nibble(digits[index + 1])
}

const fn nibble(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        b'A'..=b'F' => digit - b'A' + 10,
        _ => panic!("palette stops must look like #rrggbb"),
    }
}

fn unit(channels: Rgb) -> [f64; 3] {
    channels.map(|channel| f64::from(channel) / 255.0)
}

fn to_hex(channels: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", channels[0], channels[1], channels[2])
}

fn quantize(channels: [f64; 3]) -> Rgb {
    channels.map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn to_linear(value: f64) -> f64 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(value: f64) -> f64 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn to_oklab(channels: [f64; 3]) -> (f64, f64, f64) {
    let [red, green, blue] = channels.map(to_linear);

    let long = (0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue).cbrt();
    let medium = (0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue).cbrt();
    let short = (0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue).cbrt();

    (
        0.210_454_255_3 * long + 0.793_617_785_0 * medium - 0.004_072_046_8 * short,
        1.977_998_495_1 * long - 2.428_592_205_0 * medium + 0.450_593_709_9 * short,
        0.025_904_037_1 * long + 0.782_771_766_2 * medium - 0.808_675_766_0 * short,
    )
}

fn from_oklab(lightness: f64, a: f64, b: f64) -> [f64; 3] {
    let long = (lightness + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let medium = (lightness - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let short = (lightness - 0.089_484_177_5 * a - 1.291_485_548_0 * b).powi(3);

    [
        4.076_741_662_1 * long - 3.307_711_591_3 * medium + 0.230_969_929_2 * short,
        -1.268_438_004_6 * long + 2.609_757_401_1 * medium - 0.341_319_396_5 * short,
        -0.004_196_086_3 * long - 0.703_418_614_7 * medium + 1.707_614_701_0 * short,
    ]
    .map(from_linear)
}
