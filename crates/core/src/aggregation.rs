use crate::{ContributionDay, GithubData, OutputKind, RepositoryLanguage};

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
    pub mode: StreakMode,
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
    Wakatime(CodingActivitySummary),
    Status {
        state: &'static str,
    },
}

pub fn aggregate_card_data(data: &GithubData, output: OutputKind) -> CardData {
    match output {
        OutputKind::Dashboard => CardData::Dashboard {
            stats: aggregate_stats(data),
            languages: aggregate_languages(&data.languages, 8),
            streak: calculate_streak(&data.contributions, StreakMode::Daily, &[]),
        },
        OutputKind::Stats => CardData::Stats(aggregate_stats(data)),
        OutputKind::Languages => CardData::Languages(aggregate_languages(&data.languages, 8)),
        OutputKind::Streak => CardData::Streak(calculate_streak(
            &data.contributions,
            StreakMode::Daily,
            &[],
        )),
        OutputKind::Wakatime | OutputKind::WakatimeReadme => {
            CardData::Wakatime(aggregate_coding_activity(Vec::new(), 8, &[], false))
        }
        _ => CardData::Status { state: "ready" },
    }
}

pub fn aggregate_stats(data: &GithubData) -> AggregatedStats {
    let stats = &data.stats;
    let score = stats.stars
        + stats.commits
        + stats.pull_requests * 4
        + stats.issues * 3
        + stats.reviews * 2
        + stats.contributed_to * 2;

    AggregatedStats {
        total_stars: stats.stars,
        total_commits: stats.commits,
        total_pull_requests: stats.pull_requests,
        total_issues: stats.issues,
        total_reviews: stats.reviews,
        contributed_to: stats.contributed_to,
        score,
        rank: rank_for_score(score),
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
) -> StreakSummary {
    let mut active_ordinals = contributions
        .iter()
        .filter(|day| day.count > 0)
        .filter_map(|day| date_to_ordinal(&day.date).map(|ordinal| (ordinal, day.count)))
        .filter(|(ordinal, _)| !excluded_weekdays.contains(&weekday(*ordinal)))
        .map(|(ordinal, _)| match mode {
            StreakMode::Daily => ordinal,
            StreakMode::Weekly => ordinal / 7,
        })
        .collect::<Vec<_>>();

    active_ordinals.sort_unstable();
    active_ordinals.dedup();

    let longest = longest_run(&active_ordinals);
    let current = current_run(&active_ordinals);

    StreakSummary {
        current,
        longest,
        total_active_days: active_ordinals.len() as u32,
        mode,
    }
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

fn rank_for_score(score: u64) -> &'static str {
    match score {
        0..=99 => "C",
        100..=299 => "B",
        300..=699 => "A",
        700..=1499 => "A+",
        _ => "S",
    }
}

fn percentage_basis_points(value: u64, total: u64) -> u32 {
    if total == 0 {
        0
    } else {
        ((value * 10_000) / total) as u32
    }
}

fn longest_run(values: &[i32]) -> u32 {
    if values.is_empty() {
        return 0;
    }

    let mut longest = 1_u32;
    let mut current = 1_u32;

    for pair in values.windows(2) {
        if pair[1] == pair[0] + 1 {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 1;
        }
    }

    longest
}

fn current_run(values: &[i32]) -> u32 {
    if values.is_empty() {
        return 0;
    }

    let mut current = 1_u32;

    for pair in values.windows(2).rev() {
        if pair[1] == pair[0] + 1 {
            current += 1;
        } else {
            break;
        }
    }

    current
}

fn mask_seconds(seconds: u64) -> u64 {
    seconds / 3600 * 3600
}

fn date_to_ordinal(date: &str) -> Option<i32> {
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<i32>().ok()?;
    let day = parts.next()?.parse::<i32>().ok()?;
    Some(year * 372 + month * 31 + day)
}

fn weekday(ordinal: i32) -> u8 {
    ordinal.rem_euclid(7) as u8
}
