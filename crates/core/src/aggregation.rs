use crate::{
    Author, ContributionDay, DEFAULT_ACTIVITY_WINDOWS, DayBucket, GithubData, HeatRing, HeatWindow,
    LanguageLines, LineCounts, LineTotals, MAX_ACTIVITY_WINDOW, MEASURE_AGENT, MEASURE_EDITOR,
    MEASURE_IMPORTED, OutputKind, RepositoryLanguage, TimeBucket, TokenUsage,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedStats {
    pub total_stars: u64,
    pub total_commits: u64,
    pub total_pull_requests: u64,
    pub total_issues: u64,
    pub total_reviews: u64,
    pub contributed_to: u64,
    pub score: u64,
    pub rank: &'static str,
    pub percentile_basis_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageShare {
    pub name: String,
    pub size: u64,
    pub percentage_basis_points: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreakMode {
    Daily,
    Weekly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreakSummary {
    pub current: u32,
    pub longest: u32,
    pub total_active_days: u32,
    pub total_contributions: u64,
    pub current_start: Option<String>,
    pub current_end: Option<String>,
    pub longest_start: Option<String>,
    pub longest_end: Option<String>,
    pub mode: StreakMode,
    pub recent_daily_counts: Vec<u32>,
    pub window_start: Option<String>,
    pub window_end: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodingActivityEntry {
    pub language: String,
    pub seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodingActivitySummary {
    pub entries: Vec<CodingActivityEntry>,
    pub total_seconds: u64,
    pub masked_total_seconds: Option<u64>,
}

/// A measure of time, named rather than chosen from a fixed list.
///
/// The record holds measures by name because which ones exist depends on what was
/// installed where the work happened, and because they overlap: an agent changing
/// code while its operator watches is both agent time and editor time. A card
/// therefore names the one measure it is reporting. It never adds two, which
/// would produce a figure larger than the day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityMeasure(String);

impl Default for ActivityMeasure {
    fn default() -> Self {
        Self(MEASURE_AGENT.to_owned())
    }
}

impl ActivityMeasure {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// What the card calls this measure. The measures the project collects itself
    /// get a phrase that says what they mean; anything else is shown under the
    /// name it was recorded with, since only whoever recorded it knows.
    pub fn label(&self) -> String {
        match self.0.as_str() {
            MEASURE_AGENT => "Agent time".to_owned(),
            MEASURE_EDITOR => "Editor time".to_owned(),
            MEASURE_IMPORTED => "Imported time".to_owned(),
            other => format!("{} time", capitalise(other)),
        }
    }

    fn bucket(&self, day: &DayBucket) -> TimeBucket {
        day.measure(&self.0)
    }
}

fn days_word(count: u32) -> &'static str {
    if count == 1 { "day" } else { "days" }
}

fn capitalise(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

/// How far back a window reaches.
///
/// `All` exists because the two questions worth asking of a record are "what am I
/// doing now" and "what have I done", and the second has no day count. Expressing
/// it as a very large number of days would work arithmetically and then lie in
/// the label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivitySpan {
    Days(u32),
    All,
}

impl ActivitySpan {
    /// Reads `30` or `all`.
    pub fn parse(value: &str) -> Option<Self> {
        let text = value.trim();
        if text.eq_ignore_ascii_case("all") {
            return Some(Self::All);
        }
        text.parse::<u32>()
            .ok()
            .filter(|days| (1..=MAX_ACTIVITY_WINDOW).contains(days))
            .map(Self::Days)
    }

    pub fn as_string(self) -> String {
        match self {
            Self::Days(days) => days.to_string(),
            Self::All => "all".to_owned(),
        }
    }

    /// What the card calls this span.
    pub fn label(self) -> String {
        match self {
            Self::Days(days) => format!("Last {days} days"),
            Self::All => "All time".to_owned(),
        }
    }

    /// Days back the span reaches, for ordering one span against another.
    pub(crate) fn reach(self) -> u32 {
        match self {
            Self::Days(days) => days,
            Self::All => u32::MAX,
        }
    }
}

/// A span of days ending on one date, summed.
///
/// `active_days` is carried beside the span because the two are rarely equal and
/// the difference is the only thing that makes the totals honest. A ninety day
/// window holding thirty days of work is not a quiet quarter; it is usually a
/// record that only reaches back thirty days. Showing both lets a reader tell
/// those apart instead of reading a short history as an idle one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityWindow {
    pub span: ActivitySpan,
    /// First and last day holding work in this span. Empty when none does.
    pub start: String,
    pub end: String,
    pub active_days: u32,
    pub seconds: u64,
    pub sessions: u32,
    pub commits: LineCounts,
    pub lines: LineTotals,
    pub tokens: BTreeMap<String, TokenUsage>,
    pub requests: u32,
}

impl ActivityWindow {
    /// How many days of the span recorded work, phrased for the line under the
    /// figure.
    ///
    /// Every place this is drawn names the span beside it, so it counts the
    /// active days without repeating the span's length: "30 of 30 days" under a
    /// heading that already says thirty says thirty three times and tells the
    /// reader nothing on the second and third.
    pub fn coverage(&self) -> String {
        match self.span {
            ActivitySpan::Days(days) if self.active_days == days => "every day active".to_owned(),
            _ => format!(
                "{} {} active",
                self.active_days,
                days_word(self.active_days)
            ),
        }
    }

    /// The model that wrote the most lines in the span, if any did.
    pub fn leading_model(&self) -> Option<(&str, u64)> {
        self.lines.models().into_iter().next()
    }

    /// Models that wrote anything, largest first.
    pub fn models(&self) -> Vec<(&str, u64)> {
        self.lines.models()
    }

    /// Tokens by model, most spent first.
    pub fn token_spend(&self) -> Vec<(&str, u64)> {
        let mut ranked = self
            .tokens
            .iter()
            .map(|(model, usage)| (model.as_str(), usage.billed()))
            .filter(|(_, billed)| *billed > 0)
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
        ranked
    }
}

/// One language's standing in both windows, so a card can draw the recent share
/// against the longer one rather than showing two lists to be compared by eye.
///
/// `lines` is the same language's authorship split. It comes from a different
/// reading than the seconds do — time is measured in sittings, lines are counted
/// as they land — so a language can have time without lines on days collected
/// before the split was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityLanguage {
    pub name: String,
    pub recent_seconds: u64,
    pub recent_basis_points: u32,
    pub baseline_seconds: u64,
    pub baseline_basis_points: u32,
    /// Authorship of this language's lines in the recent span.
    pub lines: LanguageLines,
}

/// Two windows over the same record, with the languages joined across them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityComparison {
    pub measure: ActivityMeasure,
    pub recent: ActivityWindow,
    pub baseline: ActivityWindow,
    pub languages: Vec<ActivityLanguage>,
}

impl ActivityComparison {
    /// Two empty windows, for a card asked for without a record behind it. The
    /// card draws the spans it was asked about and says the record is empty,
    /// rather than drawing zeroes that look like a quiet quarter.
    pub fn empty(measure: ActivityMeasure, spans: [ActivitySpan; 2]) -> Self {
        let blank = |span| ActivityWindow {
            span,
            start: String::new(),
            end: String::new(),
            active_days: 0,
            seconds: 0,
            sessions: 0,
            commits: LineCounts::default(),
            lines: LineTotals::default(),
            tokens: BTreeMap::new(),
            requests: 0,
        };
        Self {
            measure,
            recent: blank(spans[0]),
            baseline: blank(spans[1]),
            languages: Vec::new(),
        }
    }

    /// Whether the record holds any time at all under the measure being read. A
    /// card that would otherwise draw a row of zeroes says so instead.
    pub fn is_empty(&self) -> bool {
        self.recent.seconds == 0 && self.baseline.seconds == 0
    }

    /// Whether any language in the recent span knows who wrote its lines. Days
    /// collected before the split was recorded have time but no authorship, and a
    /// card drawing a split bar over those would be inventing one.
    pub fn knows_authorship(&self) -> bool {
        self.languages
            .iter()
            .any(|language| language.lines.total() > 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardData {
    Dashboard {
        stats: AggregatedStats,
        languages: Vec<LanguageShare>,
        streak: StreakSummary,
    },
    Stats(AggregatedStats),
    Languages(Vec<LanguageShare>),
    Streak(StreakSummary),
    /// The ring on its own, for a README that composes its own row of tiles.
    Heat(StreakSummary),
    /// A single figure. Carries both sources because which one it reads is a
    /// render-time choice, and the label that goes with it belongs to the
    /// renderer rather than to aggregation.
    Metric {
        stats: AggregatedStats,
        streak: StreakSummary,
    },
    /// Two spans of the local activity record, compared. Unlike every other
    /// card, this one is not drawn from what GitHub reports: the record is
    /// collected on the machines the work happened on, so it arrives separately.
    /// Boxed because it is much the largest thing a card can carry, and an enum
    /// is as big as its widest arm: every other card would pay for this one.
    Activity(Box<ActivityComparison>),
    Status {
        state: &'static str,
    },
}

pub fn aggregate_card_data(data: &GithubData, output: OutputKind, ring: &HeatRing) -> CardData {
    match output {
        OutputKind::Dashboard => CardData::Dashboard {
            stats: aggregate_stats(data),
            languages: aggregate_languages(&data.languages, 8),
            streak: calculate_streak(&data.contributions, StreakMode::Daily, &[], ring),
        },
        OutputKind::Stats => CardData::Stats(aggregate_stats(data)),
        OutputKind::Languages => CardData::Languages(aggregate_languages(&data.languages, 8)),
        OutputKind::Streak => CardData::Streak(calculate_streak(
            &data.contributions,
            StreakMode::Daily,
            &[],
            ring,
        )),
        OutputKind::Heat => CardData::Heat(calculate_streak(
            &data.contributions,
            StreakMode::Daily,
            &[],
            ring,
        )),
        OutputKind::Metric => CardData::Metric {
            stats: aggregate_stats(data),
            streak: calculate_streak(&data.contributions, StreakMode::Daily, &[], ring),
        },
        // The activity card reads the local record rather than the profile, and
        // nothing here has one. A caller with a record builds the card from
        // `compare_activity` instead; this is the honest empty case.
        OutputKind::Activity | OutputKind::ActivityReadme => CardData::Activity(Box::new(
            ActivityComparison::empty(ActivityMeasure::default(), DEFAULT_ACTIVITY_WINDOWS),
        )),
        _ => CardData::Status { state: "ready" },
    }
}

pub fn aggregate_stats(data: &GithubData) -> AggregatedStats {
    let stats = &data.stats;
    let score = stats
        .stars
        .saturating_add(stats.commits)
        .saturating_add(stats.pull_requests.saturating_mul(4))
        .saturating_add(stats.issues.saturating_mul(3))
        .saturating_add(stats.reviews.saturating_mul(2))
        .saturating_add(stats.contributed_to.saturating_mul(2));
    let (rank, percentile_basis_points) = rank_for_stats(data);

    AggregatedStats {
        total_stars: stats.stars,
        total_commits: stats.commits,
        total_pull_requests: stats.pull_requests,
        total_issues: stats.issues,
        total_reviews: stats.reviews,
        contributed_to: stats.contributed_to,
        score,
        rank,
        percentile_basis_points,
    }
}

pub fn aggregate_languages(languages: &[RepositoryLanguage], limit: usize) -> Vec<LanguageShare> {
    let mut merged = Vec::<RepositoryLanguage>::new();

    for language in languages {
        if let Some(existing) = merged.iter_mut().find(|item| item.name == language.name) {
            existing.size += language.size;
        } else {
            merged.push(language.clone());
        }
    }

    merged.sort_by(|left, right| {
        right
            .size
            .cmp(&left.size)
            .then_with(|| left.name.cmp(&right.name))
    });

    let total = merged.iter().map(|language| language.size).sum::<u64>();

    merged
        .into_iter()
        .take(limit)
        .map(|language| LanguageShare {
            percentage_basis_points: percentage_basis_points(language.size, total),
            name: language.name,
            size: language.size,
        })
        .collect()
}

pub fn calculate_streak(
    contributions: &[ContributionDay],
    mode: StreakMode,
    excluded_weekdays: &[u8],
    ring: &HeatRing,
) -> StreakSummary {
    let days = normalized_days(contributions);
    let total_contributions = days.iter().map(|(_, count)| u64::from(*count)).sum();
    let total_active_days = days.iter().filter(|(_, count)| *count > 0).count() as u32;
    let (current, longest) = match mode {
        StreakMode::Daily => daily_streak(&days, excluded_weekdays),
        StreakMode::Weekly => weekly_streak(&days),
    };

    let anchor = match ring.window {
        HeatWindow::Streak => current.end,
        HeatWindow::Fixed(_) => days.last().map(|(ordinal, _)| *ordinal),
    };
    let span = ring.span(current.length);
    let window = daily_window(&days, anchor, span);
    let window_end = anchor.filter(|_| !window.is_empty());
    let window_start = window_end.map(|ordinal| ordinal - (window.len() as i32 - 1));

    StreakSummary {
        current: current.length,
        longest: longest.length,
        total_active_days,
        total_contributions,
        current_start: current.start.map(date_from_ordinal),
        current_end: current.end.map(date_from_ordinal),
        longest_start: longest.start.map(date_from_ordinal),
        longest_end: longest.end.map(date_from_ordinal),
        mode,
        recent_daily_counts: window,
        window_start: window_start.map(date_from_ordinal),
        window_end: window_end.map(date_from_ordinal),
    }
}

fn daily_window(days: &[(i32, u32)], anchor: Option<i32>, window: u32) -> Vec<u32> {
    let Some(last_ordinal) = anchor else {
        return Vec::new();
    };
    if window == 0 {
        return Vec::new();
    }

    let first_ordinal = last_ordinal - (window as i32 - 1);
    let mut counts = vec![0; window as usize];

    for (ordinal, count) in days {
        if *ordinal >= first_ordinal && *ordinal <= last_ordinal {
            counts[(ordinal - first_ordinal) as usize] = *count;
        }
    }

    counts
}

pub fn aggregate_coding_activity(
    entries: Vec<CodingActivityEntry>,
    limit: usize,
    ignored_languages: &[String],
    show_masked_time: bool,
) -> CodingActivitySummary {
    let mut merged = Vec::<CodingActivityEntry>::new();

    for entry in entries {
        if ignored_languages
            .iter()
            .any(|ignored| ignored == &entry.language)
        {
            continue;
        }
        if let Some(existing) = merged
            .iter_mut()
            .find(|item| item.language == entry.language)
        {
            existing.seconds += entry.seconds;
        } else {
            merged.push(entry);
        }
    }

    merged.sort_by(|left, right| {
        right
            .seconds
            .cmp(&left.seconds)
            .then_with(|| left.language.cmp(&right.language))
    });

    let total_seconds = merged.iter().map(|entry| entry.seconds).sum();
    let entries = merged.into_iter().take(limit).collect::<Vec<_>>();

    CodingActivitySummary {
        entries,
        total_seconds,
        masked_total_seconds: show_masked_time.then_some(mask_seconds(total_seconds)),
    }
}

/// Reads a record twice over two spans ending on the same day, and joins the
/// languages so the shorter span can be drawn against the longer one.
///
/// The spans are inclusive of `as_of`, so thirty days means today and the
/// twenty-nine before it. Days outside every span cost nothing but a comparison,
/// which keeps this linear in the record rather than in the spans.
pub fn compare_activity(
    days: &[DayBucket],
    measure: ActivityMeasure,
    spans: [ActivitySpan; 2],
    as_of: Option<&str>,
    limit: usize,
    ignored_languages: &[String],
) -> ActivityComparison {
    let end = as_of
        .and_then(date_to_ordinal)
        .unwrap_or_else(today_ordinal);
    let recent = window(days, &measure, spans[0], end, ignored_languages);
    let baseline = window(days, &measure, spans[1], end, ignored_languages);

    ActivityComparison {
        measure,
        languages: join_languages(&recent.1, &baseline.1, limit),
        recent: recent.0,
        baseline: baseline.0,
    }
}

/// Per-language totals for one window: time under the measure being read, and the
/// authorship of the lines written in that language.
#[derive(Default)]
struct LanguageTotals {
    seconds: BTreeMap<String, u64>,
    lines: BTreeMap<String, LanguageLines>,
}

/// Sums one span, returning the window and its per-language totals.
fn window(
    days: &[DayBucket],
    measure: &ActivityMeasure,
    span: ActivitySpan,
    end: i32,
    ignored_languages: &[String],
) -> (ActivityWindow, LanguageTotals) {
    let start = match span {
        // Inclusive of the last day, so thirty days means today and the
        // twenty-nine before it.
        ActivitySpan::Days(days) => end.saturating_sub(i32::try_from(days).unwrap_or(i32::MAX) - 1),
        ActivitySpan::All => i32::MIN,
    };

    let ignored = |language: &String| ignored_languages.iter().any(|item| item == language);

    let mut seconds = 0u64;
    let mut sessions = 0u32;
    let mut active_days = 0u32;
    let mut first = None::<&str>;
    let mut last = None::<&str>;
    let mut commits = LineCounts::default();
    let mut lines = LineTotals::default();
    let mut tokens = BTreeMap::<String, TokenUsage>::new();
    let mut requests = 0u32;
    let mut totals = LanguageTotals::default();

    for day in days {
        let Some(ordinal) = date_to_ordinal(&day.date) else {
            continue;
        };
        if ordinal < start || ordinal > end {
            continue;
        }
        let bucket = measure.bucket(day);
        if bucket.seconds > 0 {
            active_days += 1;
            // The span's own edges are what a reader wants dated, not the edges of
            // the arithmetic: an all-time window reaching back to the epoch should
            // report the first day that holds work.
            if first.is_none() {
                first = Some(&day.date);
            }
            last = Some(&day.date);
        }
        seconds += bucket.seconds;
        sessions = sessions.saturating_add(bucket.sessions);
        requests = requests.saturating_add(day.requests);
        commits.absorb(&day.commits);
        lines.absorb_facts(&day.lines);
        for (model, usage) in &day.tokens {
            tokens.entry(model.clone()).or_default().absorb(usage);
        }
        for (language, count) in &bucket.languages {
            if ignored(language) {
                continue;
            }
            *totals.seconds.entry(language.clone()).or_default() += count;
        }
        for fact in &day.lines {
            // Lines whose language went unrecorded are kept under the empty name
            // they were recorded with, so that a per-language view adds up to the
            // same figure as a view that does not ask about languages. Dropping
            // them here would leave two blocks of the same chart disagreeing by
            // however many lines came from files without a telling extension.
            if ignored(&fact.language) {
                continue;
            }
            let held = totals.lines.entry(fact.language.clone()).or_default();
            match fact.author {
                Author::Agent => held.agent += fact.total(),
                Author::Human => held.human += fact.total(),
            }
        }
    }

    let window = ActivityWindow {
        span,
        start: first.unwrap_or_default().to_owned(),
        end: last.unwrap_or_default().to_owned(),
        active_days,
        seconds,
        sessions,
        commits,
        lines,
        tokens,
        requests,
    };
    (window, totals)
}

/// Puts the two windows' language totals side by side, ordered by the recent
/// share so the card leads with what is being worked on now, and the longer span
/// reads as the baseline it is being compared against.
fn join_languages(
    recent: &LanguageTotals,
    baseline: &LanguageTotals,
    limit: usize,
) -> Vec<ActivityLanguage> {
    let recent_total = recent.seconds.values().sum::<u64>();
    let baseline_total = baseline.seconds.values().sum::<u64>();

    // A language can appear in one of these and not the other: a source may know
    // what was written without knowing how long it took, or the reverse. Taking
    // the union means neither kind of knowledge silently discards the other.
    let mut joined = recent
        .seconds
        .keys()
        .chain(baseline.seconds.keys())
        .chain(recent.lines.keys())
        .chain(baseline.lines.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|name| {
            let recent_seconds = recent.seconds.get(name).copied().unwrap_or_default();
            let baseline_seconds = baseline.seconds.get(name).copied().unwrap_or_default();
            ActivityLanguage {
                name: name.clone(),
                recent_seconds,
                recent_basis_points: percentage_basis_points(recent_seconds, recent_total),
                baseline_seconds,
                baseline_basis_points: percentage_basis_points(baseline_seconds, baseline_total),
                lines: recent.lines.get(name).copied().unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();

    joined.sort_by(|left, right| {
        right
            .recent_seconds
            .cmp(&left.recent_seconds)
            .then_with(|| right.baseline_seconds.cmp(&left.baseline_seconds))
            .then_with(|| left.name.cmp(&right.name))
    });
    joined.truncate(limit);
    joined
}

fn rank_for_stats(data: &GithubData) -> (&'static str, u32) {
    let stats = &data.stats;
    let commits_median = 250.0;
    let total_weight = 12.0;
    let rank = 1.0
        - (2.0 * exponential_cdf(stats.commits as f64 / commits_median)
            + 3.0 * exponential_cdf(stats.pull_requests as f64 / 50.0)
            + exponential_cdf(stats.issues as f64 / 25.0)
            + exponential_cdf(stats.reviews as f64 / 2.0)
            + 4.0 * log_normal_cdf(stats.stars as f64 / 50.0)
            + log_normal_cdf(data.profile.followers as f64 / 10.0))
            / total_weight;
    let percentile = (rank * 100.0).clamp(0.0, 100.0);
    let basis_points = (percentile * 100.0).round() as u32;

    let label = if percentile <= 1.0 {
        "S"
    } else if percentile <= 12.5 {
        "A+"
    } else if percentile <= 25.0 {
        "A"
    } else if percentile <= 37.5 {
        "A-"
    } else if percentile <= 50.0 {
        "B+"
    } else if percentile <= 62.5 {
        "B"
    } else if percentile <= 75.0 {
        "B-"
    } else if percentile <= 87.5 {
        "C+"
    } else {
        "C"
    };

    (label, basis_points)
}

fn exponential_cdf(value: f64) -> f64 {
    1.0 - 2_f64.powf(-value)
}

fn log_normal_cdf(value: f64) -> f64 {
    value / (1.0 + value)
}

fn percentage_basis_points(value: u64, total: u64) -> u32 {
    value.saturating_mul(10_000).checked_div(total).unwrap_or(0) as u32
}

fn normalized_days(contributions: &[ContributionDay]) -> Vec<(i32, u32)> {
    let today = today_ordinal();
    let tomorrow = today + 1;
    let mut days = contributions
        .iter()
        .filter_map(|day| {
            let ordinal = date_to_ordinal(&day.date)?;
            if ordinal <= today || (ordinal == tomorrow && day.count > 0) {
                Some((ordinal, day.count))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    days.sort_unstable_by_key(|(ordinal, _)| *ordinal);
    days
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StreakRun {
    length: u32,
    start: Option<i32>,
    end: Option<i32>,
}

fn daily_streak(days: &[(i32, u32)], excluded_weekdays: &[u8]) -> (StreakRun, StreakRun) {
    let mut current = StreakRun::default();
    let mut longest = StreakRun::default();
    let Some((last_day, _)) = days.last() else {
        return (current, longest);
    };

    for (ordinal, count) in days {
        if *count > 0 || (current.length > 0 && excluded_weekdays.contains(&weekday(*ordinal))) {
            current.length += 1;
            current.start = current.start.or(Some(*ordinal));
            current.end = Some(*ordinal);
            if current.length > longest.length {
                longest = current;
            }
        } else if ordinal != last_day {
            current = StreakRun::default();
        }
    }

    (current, longest)
}

fn weekly_streak(days: &[(i32, u32)]) -> (StreakRun, StreakRun) {
    let mut by_week = BTreeMap::<i32, u32>::new();
    for (ordinal, count) in days {
        let week = sunday_of_week(*ordinal);
        *by_week.entry(week).or_default() += *count;
    }

    let mut current = StreakRun::default();
    let mut longest = StreakRun::default();
    let Some(first_week) = by_week.keys().next().copied() else {
        return (current, longest);
    };
    let Some(last_week) = by_week.keys().next_back().copied() else {
        return (current, longest);
    };

    let mut week = first_week;
    while week <= last_week {
        let count = by_week.get(&week).copied().unwrap_or(0);
        if count > 0 {
            current.length += 1;
            current.start = current.start.or(Some(week));
            current.end = Some(week);
            if current.length > longest.length {
                longest = current;
            }
        } else if week != last_week {
            current = StreakRun::default();
        }
        week += 7;
    }

    (current, longest)
}

fn mask_seconds(seconds: u64) -> u64 {
    seconds / 3600 * 3600
}

fn date_to_ordinal(date: &str) -> Option<i32> {
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    days_from_civil(year, month, day)
}

fn weekday(ordinal: i32) -> u8 {
    (ordinal + 4).rem_euclid(7) as u8
}

fn sunday_of_week(ordinal: i32) -> i32 {
    ordinal - i32::from(weekday(ordinal))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i32> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let adjusted_year = year - i32::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let month = month as i32;
    let day = day as i32;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

fn date_from_ordinal(ordinal: i32) -> String {
    let days = ordinal + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + i32::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn today_ordinal() -> i32 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    (seconds / 86_400) as i32
}
