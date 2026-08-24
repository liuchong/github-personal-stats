use std::collections::BTreeMap;

use github_personal_stats_core::TimeBucket;

/// A moment at which something happened, with the languages it touched. The
/// weights say how to divide that moment's time when it touched more than one.
pub struct Event {
    pub second: i64,
    pub day: String,
    pub languages: Vec<(&'static str, u64)>,
}

impl Event {
    pub fn new(second: i64, day: impl Into<String>, language: &'static str) -> Self {
        Self {
            second,
            day: day.into(),
            languages: vec![(language, 1)],
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

    let weight: u64 = event.languages.iter().map(|(_, weight)| weight).sum();
    if weight == 0 {
        return;
    }

    let mut spent = 0;
    for (language, share) in &event.languages {
        let slice = seconds * share / weight;
        *bucket.languages.entry((*language).to_string()).or_default() += slice;
        spent += slice;
    }

    // Integer division loses a second here and there. Give the remainder to the
    // language that took the largest share, so the parts equal the whole.
    if let Some((language, _)) = event.languages.first() {
        *bucket.languages.entry((*language).to_string()).or_default() += seconds - spent;
    }
}
