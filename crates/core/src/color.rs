use crate::GithubStatsError;

pub const HEAT_RAMP_STEPS: usize = 4;

const NAMED_RAMPS: [(&str, [&str; HEAT_RAMP_STEPS]); 6] = [
    ("heat-orange", ["#ffe3ad", "#ffc65c", "#ffa726", "#fb8c00"]),
    ("github-blue", ["#cfe0fa", "#8fbdf2", "#3b82d6", "#0b69d4"]),
    ("forest", ["#d3ecd5", "#9fd6a6", "#4caf67", "#2e7d32"]),
    ("violet", ["#e5dbf7", "#c0a7ee", "#8a63d2", "#6532c4"]),
    ("crimson", ["#fbd7db", "#f3a3ac", "#e0576c", "#c62443"]),
    ("graphite", ["#e4e8ee", "#c2c9d3", "#8b95a3", "#5b6572"]),
];

pub fn named_ramps() -> impl Iterator<Item = &'static str> {
    NAMED_RAMPS.iter().map(|(name, _)| *name)
}

pub fn parse_heat_ramp(value: &str) -> Result<Vec<String>, GithubStatsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid("a colour name, one hex value, or four hex values"));
    }

    if let Some((_, ramp)) = NAMED_RAMPS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(trimmed))
    {
        return Ok(ramp.iter().map(|stop| (*stop).to_owned()).collect());
    }

    let stops = trimmed
        .split(',')
        .map(str::trim)
        .filter(|stop| !stop.is_empty())
        .collect::<Vec<_>>();

    match stops.len() {
        1 => Ok(derive_ramp(parse_hex(stops[0])?)),
        HEAT_RAMP_STEPS => stops
            .into_iter()
            .map(|stop| parse_hex(stop).map(|_| normalize_hex(stop)))
            .collect(),
        _ => Err(invalid(
            "expected a colour name, one hex value, or four comma separated hex values",
        )),
    }
}

fn derive_ramp(seed: [f64; 3]) -> Vec<String> {
    let (lightness, a, b) = to_oklab(seed);
    let chroma = a.hypot(b);
    let hue = b.atan2(a);
    let lightest = lightness.max(0.55) + (1.0 - lightness.max(0.55)) * 0.82;

    (0..HEAT_RAMP_STEPS)
        .map(|index| {
            let progress = index as f64 / (HEAT_RAMP_STEPS - 1) as f64;
            let step_lightness = lightest + (lightness - lightest) * progress;
            let step_chroma = chroma * (0.28 + 0.72 * progress);
            to_hex(from_oklab(
                step_lightness,
                step_chroma * hue.cos(),
                step_chroma * hue.sin(),
            ))
        })
        .collect()
}

fn invalid(message: &str) -> GithubStatsError {
    GithubStatsError::InvalidConfig {
        field: "heat_color",
        message: message.to_owned(),
    }
}

fn parse_hex(value: &str) -> Result<[f64; 3], GithubStatsError> {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if digits.len() != 6 || !digits.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(invalid("hex values must look like #rrggbb"));
    }

    let channel = |start: usize| {
        u8::from_str_radix(&digits[start..start + 2], 16)
            .map(|value| f64::from(value) / 255.0)
            .map_err(|_| invalid("hex values must look like #rrggbb"))
    };

    Ok([channel(0)?, channel(2)?, channel(4)?])
}

fn normalize_hex(value: &str) -> String {
    let digits = value.strip_prefix('#').unwrap_or(value);
    format!("#{}", digits.to_ascii_lowercase())
}

fn to_hex(channels: [f64; 3]) -> String {
    let byte = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(channels[0]),
        byte(channels[1]),
        byte(channels[2])
    )
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
