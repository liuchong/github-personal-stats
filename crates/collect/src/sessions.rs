use std::collections::BTreeMap;

use github_personal_stats_core::{Author, TimeBucket};

/// A moment at which something happened, with what it touched. The weights say
/// how to divide that moment's time when it touched more than one thing.
///
/// A part carries an author where the source knew one. Where it did not, the
/// moment still counts towards its language and towards the day; only the
/// attribution is missing, and a missing attribution is recorded as missing
/// rather than guessed at.
pub struct Event {
    pub second: i64,
    pub day: String,
    pub languages: Vec<Part>,
}

/// A share of one moment: what was touched, by whom and with what if known, and
/// how much of the moment it accounts for.
pub struct Part {
    pub language: &'static str,
    pub author: Option<Author>,
    /// Empty where the source did not name one, which is the usual case outside
    /// an agent's own writes.
    pub model: String,
    pub weight: u64,
}

impl Part {
    pub fn new(language: &'static str, author: Option<Author>, weight: u64) -> Self {
        Self {
            language,
            author,
            model: String::new(),
            weight,
        }
    }

    pub fn by(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

impl Event {
    pub fn new(second: i64, day: impl Into<String>, language: &'static str) -> Self {
        Self {
            second,
            day: day.into(),
            languages: vec![Part::new(language, None, 1)],
        }
    }
}

/// Turns moments into time worked, by one rule: a gap no longer than the idle
/// timeout counts as time, a longer gap ends the session. Editor time and agent
/// time both come through here, so a difference between them is a difference in
/// what was observed rather than in how it was counted.
///
/// Events must be sorted by second. Time lands on the earlier moment of a pair,
/// because that is the moment we know someone was there.
pub fn accumulate(events: &[Event], idle_timeout_seconds: i64) -> BTreeMap<String, TimeBucket> {
    let mut days = BTreeMap::<String, TimeBucket>::new();
    let mut previous: Option<&Event> = None;

    for event in events {
        let starts_session = match previous {
            None => true,
            Some(earlier) => {
                let gap = event.second - earlier.second;
                if gap > 0 && gap <= idle_timeout_seconds {
                    spend(&mut days, earlier, gap.unsigned_abs());
                    false
                } else {
                    gap > idle_timeout_seconds
                }
            }
        };

        if starts_session {
            days.entry(event.day.clone()).or_default().sessions += 1;
        }

        previous = Some(event);
    }

    days
}

fn spend(days: &mut BTreeMap<String, TimeBucket>, event: &Event, seconds: u64) {
    let bucket = days.entry(event.day.clone()).or_default();
    bucket.seconds += seconds;

    let weight: u64 = event.languages.iter().map(|part| part.weight).sum();
    if weight == 0 {
        return;
    }

    let mut spent = 0;
    for part in &event.languages {
        let slice = seconds * part.weight / weight;
        *bucket
            .languages
            .entry(part.language.to_string())
            .or_default() += slice;
        if let Some(author) = part.author {
            bucket.spend(part.language, author, &part.model, slice);
        }
        spent += slice;
    }

    // Integer division loses a second here and there. Give the remainder to the
    // part that took the largest share, so the parts equal the whole.
    if let Some(part) = event.languages.first() {
        let remainder = seconds - spent;
        *bucket
            .languages
            .entry(part.language.to_string())
            .or_default() += remainder;
        if let Some(author) = part.author {
            bucket.spend(part.language, author, &part.model, remainder);
        }
    }
}
