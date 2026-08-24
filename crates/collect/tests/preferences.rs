use github_personal_stats_collect::preferences::Preferences;

#[test]
fn a_setting_is_read_by_the_name_of_the_flag_it_replaces() {
    let prefs = Preferences::parse("sink = git\nrepo = /var/storage\n");

    assert_eq!(prefs.flag("--sink"), Some("git"));
    assert_eq!(prefs.flag("sink"), Some("git"));
    assert_eq!(prefs.flag("--repo"), Some("/var/storage"));
}

#[test]
fn comments_and_blank_lines_are_not_settings() {
    let prefs = Preferences::parse(
        "# where the storage lives\n\
         \n\
         origin = git@example.com:me/data.git\n\
         # sink = file\n",
    );

    assert_eq!(prefs.flag("origin"), Some("git@example.com:me/data.git"));
    assert_eq!(prefs.flag("sink"), None);
}

#[test]
fn spacing_around_a_setting_does_not_change_it() {
    let prefs = Preferences::parse("   branch   =   history   \n");

    assert_eq!(prefs.flag("branch"), Some("history"));
}

#[test]
fn a_value_containing_an_equals_sign_survives() {
    // Remote URLs and query strings carry them.
    let prefs = Preferences::parse("origin = https://example.com/git?repo=data\n");

    assert_eq!(
        prefs.flag("origin"),
        Some("https://example.com/git?repo=data")
    );
}

#[test]
fn a_setting_left_empty_is_no_setting_at_all() {
    // Otherwise commenting a value out by deleting it would configure the empty
    // string, which is worse than not configuring anything.
    let prefs = Preferences::parse("repo =\nsink = git\n");

    assert_eq!(prefs.flag("repo"), None);
    assert_eq!(prefs.flag("sink"), Some("git"));
}

#[test]
fn a_line_that_is_not_a_setting_is_ignored_rather_than_fatal() {
    let prefs = Preferences::parse("this is not a setting\nsink = git\n");

    assert_eq!(prefs.flag("sink"), Some("git"));
}

#[test]
fn a_switch_is_on_when_written_and_off_when_denied() {
    assert!(Preferences::parse("no-push = true").switch("no-push"));
    assert!(Preferences::parse("no-push = yes").switch("no-push"));
    assert!(!Preferences::parse("no-push = false").switch("no-push"));
    assert!(!Preferences::parse("no-push = off").switch("no-push"));
    assert!(!Preferences::parse("no-push = 0").switch("no-push"));
    assert!(!Preferences::parse("").switch("no-push"));
}

#[test]
fn no_configuration_file_is_the_same_as_an_empty_one() {
    let nowhere = std::env::temp_dir().join("gps-preferences-absent");
    let _ = std::fs::remove_dir_all(&nowhere);

    let prefs = Preferences::load(&nowhere);

    assert_eq!(prefs.flag("sink"), None);
}
