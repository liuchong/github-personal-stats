use github_stats_core::workspace_info;

fn main() {
    println!("{}", workspace_info().to_json());
}
