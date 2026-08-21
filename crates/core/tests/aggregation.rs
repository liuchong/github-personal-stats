use github_personal_stats_core::{
    CodingActivityEntry, ContributionDay, GithubClient, GithubStatsConfig, HeatRing, HeatWindow,
    MockGithubClient, OutputKind, RepositoryLanguage, StreakMode, aggregate_card_data,
    aggregate_coding_activity, aggregate_languages, aggregate_stats, calculate_streak,
};

const FIXTURE: &str = include_str!("fixtures/github_user_data.json");

fn fixture_data() -> github_personal_stats_core::GithubData {
    let config = GithubStatsConfig::new("octo").unwrap();
    MockGithubClient::success(FIXTURE)
        .fetch_user_data(&config)
        .unwrap()
}

#[test]
fn stats_aggregation_computes_score_and_rank() {
    let data = fixture_data();
    let stats = aggregate_stats(&data);

    assert_eq!(stats.total_stars, 120);
    assert_eq!(stats.total_commits, 350);
    assert_eq!(stats.total_pull_requests, 21);
    assert_eq!(stats.score, 619);
    assert_eq!(stats.rank, "B+");
}

#[test]
fn stats_rank_follows_weighted_percentile_model() {
    let mut data = fixture_data();
    data.profile.followers = 189;
    data.stats.stars = 7_016;
    data.stats.commits = 292;
    data.stats.pull_requests = 7;
    data.stats.issues = 0;
    data.stats.reviews = 4;
    data.stats.contributed_to = 9;
    let stats = aggregate_stats(&data);

    assert_eq!(stats.score, 7_362);
    assert_eq!(stats.rank, "B+");
}

#[test]
fn stats_aggregation_exposes_the_percentile_behind_the_rank_label() {
    let data = fixture_data();
    let stats = aggregate_stats(&data);

    assert_eq!(stats.rank, "B+");
    assert!(
        (3_750..=5_000).contains(&stats.percentile_basis_points),
        "a B+ label must come from the 37.5%-50% band, got {}",
        stats.percentile_basis_points
    );
}

#[test]
fn recent_window_covers_a_fixed_span_ending_on_the_last_known_day() {
    let days = vec![
        ContributionDay {
            date: "2026-01-01".to_owned(),
            count: 9,
        },
        ContributionDay {
            date: "2026-05-01".to_owned(),
            count: 4,
        },
        ContributionDay {
            date: "2026-05-03".to_owned(),
            count: 2,
        },
    ];

    let ring = HeatRing {
        window: HeatWindow::Fixed(30),
        ..HeatRing::default()
    };
    let streak = calculate_streak(&days, StreakMode::Daily, &[], &ring);
    let window = streak.recent_daily_counts;

    assert_eq!(window.len(), 30);
    assert_eq!(window.last().copied(), Some(2));
    assert_eq!(window[28], 0);
    assert_eq!(window[27], 4);
    assert_eq!(
        window.iter().sum::<u32>(),
        6,
        "days older than the window are dropped"
    );
}

#[test]
fn streak_window_covers_the_streak_so_every_node_is_an_active_day() {
    let days = (1u32..=6)
        .map(|day| ContributionDay {
            date: format!("2026-05-{day:02}"),
            count: if day == 3 { 0 } else { day },
        })
        .collect::<Vec<_>>();

    let streak = calculate_streak(&days, StreakMode::Daily, &[], &HeatRing::default());

    assert_eq!(streak.current, 3);
    assert_eq!(
        streak.recent_daily_counts,
        vec![4, 5, 6],
        "the default window tracks the streak itself"
    );
}

#[test]
fn streak_window_stops_at_the_configured_limit() {
    let days = (1u32..=20)
        .map(|day| ContributionDay {
            date: format!("2026-05-{day:02}"),
            count: day,
        })
        .collect::<Vec<_>>();
    let ring = HeatRing {
        limit: Some(5),
        ..HeatRing::default()
    };

    let streak = calculate_streak(&days, StreakMode::Daily, &[], &ring);

    assert_eq!(streak.current, 20, "the streak itself is not truncated");
    assert_eq!(
        streak.recent_daily_counts,
        vec![16, 17, 18, 19, 20],
        "the ring keeps the most recent days up to the limit"
    );
}

#[test]
fn recent_window_is_empty_without_contributions() {
    let streak = calculate_streak(&[], StreakMode::Daily, &[], &HeatRing::default());

    assert!(streak.recent_daily_counts.is_empty());
}

#[test]
fn language_aggregation_merges_sorts_and_limits() {
    let languages = vec![
        RepositoryLanguage {
            name: "Rust".to_owned(),
            size: 100,
        },
        RepositoryLanguage {
            name: "TypeScript".to_owned(),
            size: 50,
        },
        RepositoryLanguage {
            name: "Rust".to_owned(),
            size: 50,
        },
    ];

    let shares = aggregate_languages(&languages, 2);

    assert_eq!(shares.len(), 2);
    assert_eq!(shares[0].name, "Rust");
    assert_eq!(shares[0].size, 150);
    assert_eq!(shares[0].percentage_basis_points, 7500);
}

#[test]
fn language_aggregation_handles_empty_input() {
    let shares = aggregate_languages(&[], 3);

    assert!(shares.is_empty());
}

#[test]
fn daily_streak_handles_gaps() {
    let days = vec![
        ContributionDay {
            date: "2026-05-10".to_owned(),
            count: 1,
        },
        ContributionDay {
            date: "2026-05-11".to_owned(),
            count: 0,
        },
        ContributionDay {
            date: "2026-05-12".to_owned(),
            count: 3,
        },
        ContributionDay {
            date: "2026-05-13".to_owned(),
            count: 2,
        },
    ];

    let streak = calculate_streak(&days, StreakMode::Daily, &[], &HeatRing::default());

    assert_eq!(streak.longest, 2);
    assert_eq!(streak.current, 2);
    assert_eq!(streak.total_active_days, 3);
    assert_eq!(streak.total_contributions, 6);
    assert_eq!(streak.current_start.as_deref(), Some("2026-05-12"));
    assert_eq!(streak.current_end.as_deref(), Some("2026-05-13"));
    assert_eq!(streak.longest_start.as_deref(), Some("2026-05-12"));
    assert_eq!(streak.longest_end.as_deref(), Some("2026-05-13"));
}

#[test]
fn weekly_streak_deduplicates_active_days_in_same_week_bucket() {
    let current_week = test_sunday_of_week(current_date_ordinal() - 1);
    let previous_week = current_week - 7;
    let days = vec![
        ContributionDay {
            date: ordinal_to_date(previous_week),
            count: 1,
        },
        ContributionDay {
            date: ordinal_to_date(previous_week + 1),
            count: 3,
        },
        ContributionDay {
            date: ordinal_to_date(current_week),
            count: 2,
        },
    ];

    let streak = calculate_streak(&days, StreakMode::Weekly, &[], &HeatRing::default());
    let expected_start = ordinal_to_date(previous_week);
    let expected_end = ordinal_to_date(current_week);

    assert_eq!(streak.total_active_days, 3);
    assert_eq!(streak.total_contributions, 6);
    assert_eq!(streak.longest, 2);
    assert_eq!(
        streak.current_start.as_deref(),
        Some(expected_start.as_str())
    );
    assert_eq!(streak.current_end.as_deref(), Some(expected_end.as_str()));
}

#[test]
fn weekly_streak_breaks_when_middle_week_has_no_contributions() {
    let current_week = test_sunday_of_week(current_date_ordinal() - 1);
    let first_week = current_week - 14;
    let days = vec![
        ContributionDay {
            date: ordinal_to_date(first_week + 1),
            count: 2,
        },
        ContributionDay {
            date: ordinal_to_date(first_week + 2),
            count: 1,
        },
        ContributionDay {
            date: ordinal_to_date(current_week + 1),
            count: 4,
        },
    ];

    let streak = calculate_streak(&days, StreakMode::Weekly, &[], &HeatRing::default());
    let expected_current_week = ordinal_to_date(current_week);

    assert_eq!(streak.longest, 1);
    assert_eq!(streak.current, 1);
    assert_eq!(
        streak.current_start.as_deref(),
        Some(expected_current_week.as_str())
    );
    assert_eq!(
        streak.current_end.as_deref(),
        Some(expected_current_week.as_str())
    );
}

#[test]
fn daily_streak_keeps_yesterday_streak_when_today_is_empty() {
    let days = vec![
        ContributionDay {
            date: "2026-05-10".to_owned(),
            count: 0,
        },
        ContributionDay {
            date: "2026-05-11".to_owned(),
            count: 2,
        },
        ContributionDay {
            date: "2026-05-12".to_owned(),
            count: 1,
        },
        ContributionDay {
            date: "2026-05-13".to_owned(),
            count: 0,
        },
    ];

    let streak = calculate_streak(&days, StreakMode::Daily, &[], &HeatRing::default());

    assert_eq!(streak.current, 2);
    assert_eq!(streak.longest, 2);
    assert_eq!(streak.total_contributions, 3);
    assert_eq!(streak.current_start.as_deref(), Some("2026-05-11"));
    assert_eq!(streak.current_end.as_deref(), Some("2026-05-12"));
}

#[test]
fn streak_handles_empty_contributions() {
    let streak = calculate_streak(&[], StreakMode::Daily, &[], &HeatRing::default());

    assert_eq!(streak.current, 0);
    assert_eq!(streak.longest, 0);
    assert_eq!(streak.current_start, None);
    assert_eq!(streak.longest_start, None);
}

#[test]
fn daily_streak_ignores_far_future_days_and_keeps_tomorrow_with_activity() {
    let today = current_date_ordinal();
    let contributions = vec![
        ContributionDay {
            date: ordinal_to_date(today - 1),
            count: 1,
        },
        ContributionDay {
            date: ordinal_to_date(today),
            count: 1,
        },
        ContributionDay {
            date: ordinal_to_date(today + 1),
            count: 1,
        },
        ContributionDay {
            date: ordinal_to_date(today + 2),
            count: 100,
        },
    ];

    let streak = calculate_streak(&contributions, StreakMode::Daily, &[], &HeatRing::default());
    let expected_start = ordinal_to_date(today - 1);
    let expected_end = ordinal_to_date(today + 1);

    assert_eq!(streak.current, 3);
    assert_eq!(streak.longest, 3);
    assert_eq!(
        streak.current_start.as_deref(),
        Some(expected_start.as_str())
    );
    assert_eq!(streak.current_end.as_deref(), Some(expected_end.as_str()));
    assert_eq!(streak.total_contributions, 3);
}

fn current_date_ordinal() -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    (seconds / 86_400) as i32
}

fn ordinal_to_date(ordinal: i32) -> String {
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

fn test_sunday_of_week(ordinal: i32) -> i32 {
    ordinal - (ordinal + 4).rem_euclid(7)
}

#[test]
fn coding_activity_merges_ignores_limits_and_masks_total() {
    let ignored = vec!["Text".to_owned()];
    let summary = aggregate_coding_activity(
        vec![
            CodingActivityEntry {
                language: "Rust".to_owned(),
                seconds: 3700,
            },
            CodingActivityEntry {
                language: "Text".to_owned(),
                seconds: 900,
            },
            CodingActivityEntry {
                language: "Rust".to_owned(),
                seconds: 1800,
            },
            CodingActivityEntry {
                language: "Shell".to_owned(),
                seconds: 600,
            },
        ],
        1,
        &ignored,
        true,
    );

    assert_eq!(summary.entries.len(), 1);
    assert_eq!(summary.entries[0].language, "Rust");
    assert_eq!(summary.entries[0].seconds, 5500);
    assert_eq!(summary.total_seconds, 6100);
    assert_eq!(summary.masked_total_seconds, Some(3600));
}

#[test]
fn card_data_dashboard_reuses_shared_aggregations() {
    let data = fixture_data();
    let card = aggregate_card_data(&data, OutputKind::Dashboard, &HeatRing::default());

    match card {
        github_personal_stats_core::CardData::Dashboard {
            stats,
            languages,
            streak,
        } => {
            assert_eq!(stats.rank, "B+");
            assert_eq!(languages[0].name, "Rust");
            assert_eq!(streak.total_active_days, 2);
            assert_eq!(streak.total_contributions, 4);
        }
        _ => panic!("expected dashboard card data"),
    }
}
