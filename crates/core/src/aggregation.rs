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
    pub total_contributions: u64,
    pub current_start: Option<String>,
    pub current_end: Option<String>,
    pub longest_start: Option<String>,
    pub longest_end: Option<String>,
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
    let rank = rank_for_stats(data);
    let score = stats
        .stars
        .saturating_add(stats.commits)
        .saturating_add(stats.pull_requests.saturating_mul(4))
        .saturating_add(stats.issues.saturating_mul(3))
        .saturating_add(stats.reviews.saturating_mul(2))
        .saturating_add(stats.contributed_to.saturating_mul(2));

    AggregatedStats {
        total_stars: stats.stars,
        total_commits: stats.commits,
        total_pull_requests: stats.pull_requests,
        total_issues: stats.issues,
        total_reviews: stats.reviews,
        contributed_to: stats.contributed_to,
        score,
        rank,
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
    let days = normalized_days(contributions);
    let total_contributions = days.iter().map(|(_, count)| u64::from(*count)).sum();
    let total_active_days = days.iter().filter(|(_, count)| *count > 0).count() as u32;
    let (current, longest) = match mode {
        StreakMode::Daily => daily_streak(&days, excluded_weekdays),
        StreakMode::Weekly => weekly_streak(&days),
    };

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

fn rank_for_stats(data: &GithubData) -> &'static str {
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
    let percentile = rank * 100.0;

    if percentile <= 0.5 {
        "S+"
    } else if percentile <= 1.0 {
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
    }
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
    let mut days = contributions
        .iter()
        .filter_map(|day| date_to_ordinal(&day.date).map(|ordinal| (ordinal, day.count)))
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
    let mut weeks = Vec::<(i32, u32)>::new();
    for (ordinal, count) in days {
        let week = sunday_of_week(*ordinal);
        if let Some((_, total)) = weeks.iter_mut().find(|(existing, _)| *existing == week) {
            *total += *count;
        } else {
            weeks.push((week, *count));
        }
    }

    let mut current = StreakRun::default();
    let mut longest = StreakRun::default();
    let Some((last_week, _)) = weeks.last().copied() else {
        return (current, longest);
    };

    for (week, count) in weeks {
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
