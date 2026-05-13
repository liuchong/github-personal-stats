use github_stats_core::{
    CodingActivityEntry, ContributionDay, GithubClient, GithubStatsConfig, MockGithubClient,
    OutputKind, RepositoryLanguage, StreakMode, aggregate_card_data, aggregate_coding_activity,
    aggregate_languages, aggregate_stats, calculate_streak,
};

const FIXTURE: &str = include_str!("fixtures/github_user_data.json");

fn fixture_data() -> github_stats_core::GithubData {
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

    let streak = calculate_streak(&days, StreakMode::Daily, &[]);

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
    let days = vec![
        ContributionDay {
            date: "2026-05-10".to_owned(),
            count: 1,
        },
        ContributionDay {
            date: "2026-05-11".to_owned(),
            count: 3,
        },
        ContributionDay {
            date: "2026-05-20".to_owned(),
            count: 2,
        },
    ];

    let streak = calculate_streak(&days, StreakMode::Weekly, &[]);

    assert_eq!(streak.total_active_days, 3);
    assert_eq!(streak.total_contributions, 6);
    assert_eq!(streak.longest, 2);
    assert_eq!(streak.current_start.as_deref(), Some("2026-05-10"));
    assert_eq!(streak.current_end.as_deref(), Some("2026-05-17"));
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

    let streak = calculate_streak(&days, StreakMode::Daily, &[]);

    assert_eq!(streak.current, 2);
    assert_eq!(streak.longest, 2);
    assert_eq!(streak.total_contributions, 3);
    assert_eq!(streak.current_start.as_deref(), Some("2026-05-11"));
    assert_eq!(streak.current_end.as_deref(), Some("2026-05-12"));
}

#[test]
fn streak_handles_empty_contributions() {
    let streak = calculate_streak(&[], StreakMode::Daily, &[]);

    assert_eq!(streak.current, 0);
    assert_eq!(streak.longest, 0);
    assert_eq!(streak.current_start, None);
    assert_eq!(streak.longest_start, None);
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
    let card = aggregate_card_data(&data, OutputKind::Dashboard);

    match card {
        github_stats_core::CardData::Dashboard {
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
