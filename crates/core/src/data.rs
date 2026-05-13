#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubProfile {
    pub login: String,
    pub name: Option<String>,
    pub followers: u64,
    pub public_repositories: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStats {
    pub stars: u64,
    pub commits: u64,
    pub pull_requests: u64,
    pub issues: u64,
    pub reviews: u64,
    pub contributed_to: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryLanguage {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionDay {
    pub date: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubData {
    pub profile: GithubProfile,
    pub stats: UserStats,
    pub languages: Vec<RepositoryLanguage>,
    pub contributions: Vec<ContributionDay>,
}
