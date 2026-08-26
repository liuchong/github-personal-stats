use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CodingActivityEntry, GithubStatsError};

pub const ACTIVITY_SCHEMA: u32 = 2;

const DATE_LENGTH: usize = 10;
const TIMESTAMP_LENGTH: usize = 20;

/// Time an agent spent changing code, derived from the editor's own record of
/// what it generated.
pub const MEASURE_AGENT: &str = "agent";

/// Time the editor was being worked in, reported by editor plugins.
pub const MEASURE_EDITOR: &str = "editor";

/// A total carried in from another tracker. Named separately because such a total
/// generally counts agent work too, so it overlaps `agent` rather than
/// complementing it.
pub const MEASURE_IMPORTED: &str = "imported";

/// Stands in for a language the source did not name.
pub const UNKNOWN_LANGUAGE: &str = "";

/// What to call a language on a card or a chart.
///
/// The record leaves the name empty when its source counted lines without saying
/// what they were, which is the truthful thing to store and an unreadable thing
/// to draw: a row with no name looks like a bug. Naming it is a presentation
/// choice, so it is made here rather than written into the record.
pub fn language_label(name: &str) -> &str {
    if name == UNKNOWN_LANGUAGE {
        "unknown"
    } else {
        name
    }
}

/// Who a change came from.
///
/// Exact for a line, which has one author. Time divides by the same rule that
/// puts it against a language — a gap belongs to the moment before it, and that
/// moment had one cause — so time divides too, within a single measure. What
/// never divides is time across measures: agent time and editor time can cover
/// the same minute, and nothing sums them.
///
/// `Human` is the weaker of the two claims. It means an agent was not seen to do
/// it, which also covers a checkout, a rebase, a formatter and a scripted
/// rewrite. Only a plugin watching keystrokes can say a person typed something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Author {
    Agent,
    Human,
}

impl Author {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Human => "human",
        }
    }
}

/// One fact about lines written: in what language, by whom, and with which model.
///
/// The record keeps lines at this grain rather than pre-summed by language or by
/// model, because which of those a reader wants is not knowable when collecting.
/// Every breakdown the cards draw — by language, by model, by author, or by any
/// pair of those — is a fold over these, so a new breakdown needs no new field
/// and no new collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineFact {
    /// Empty when the source counted lines without saying what they were.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub language: String,
    pub author: Author,
    /// Empty for a person's lines, and for an agent whose model went unrecorded.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(default)]
    pub added: u64,
    #[serde(default)]
    pub deleted: u64,
}

impl LineFact {
    pub fn new(language: impl Into<String>, author: Author, model: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            author,
            model: model.into(),
            added: 0,
            deleted: 0,
        }
    }

    /// What makes two facts the same fact. Folding and merging both key on this.
    fn key(&self) -> (&str, Author, &str) {
        (&self.language, self.author, &self.model)
    }

    pub fn total(&self) -> u64 {
        self.added + self.deleted
    }
}

/// Tokens spent on one model. Collected where a source reports them; agents that
/// do not report tokens simply have none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cached: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cached
    }

    /// Tokens that were paid for, which is everything the cache did not serve.
    pub fn billed(&self) -> u64 {
        self.input + self.output
    }

    pub(crate) fn absorb(&mut self, other: &Self) {
        self.input += other.input;
        self.output += other.output;
        self.cached += other.cached;
    }

    pub(crate) fn keep_fuller(&mut self, other: &Self) {
        self.input = self.input.max(other.input);
        self.output = self.output.max(other.output);
        self.cached = self.cached.max(other.cached);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LineCounts {
    pub agent_added: u64,
    pub agent_deleted: u64,
    pub tab_added: u64,
    pub tab_deleted: u64,
    pub human_added: u64,
    pub human_deleted: u64,
    pub blank_added: u64,
    pub blank_deleted: u64,
    pub unattributed_added: u64,
    pub unattributed_deleted: u64,
}

impl LineCounts {
    pub fn ai_added(&self) -> u64 {
        self.agent_added + self.tab_added
    }

    pub fn ai_deleted(&self) -> u64 {
        self.agent_deleted + self.tab_deleted
    }

    pub fn attributed_added(&self) -> u64 {
        self.ai_added() + self.human_added
    }

    pub fn attributed_deleted(&self) -> u64 {
        self.ai_deleted() + self.human_deleted
    }

    pub fn attributed(&self) -> u64 {
        self.attributed_added() + self.attributed_deleted()
    }

    pub fn added(&self) -> u64 {
        self.attributed_added() + self.blank_added + self.unattributed_added
    }

    pub fn deleted(&self) -> u64 {
        self.attributed_deleted() + self.blank_deleted + self.unattributed_deleted
    }

    pub fn changed(&self) -> u64 {
        self.added() + self.deleted()
    }

    pub fn ai_share_basis_points(&self) -> u32 {
        let attributed = self.attributed();
        if attributed == 0 {
            return 0;
        }
        let assisted = self.ai_added() + self.ai_deleted();
        u32::try_from(assisted.saturating_mul(10_000) / attributed).unwrap_or(10_000)
    }

    pub fn attributed_share_basis_points(&self) -> u32 {
        let changed = self.changed();
        if changed == 0 {
            return 0;
        }
        u32::try_from(self.attributed().saturating_mul(10_000) / changed).unwrap_or(10_000)
    }

    pub(crate) fn absorb(&mut self, other: &Self) {
        self.agent_added += other.agent_added;
        self.agent_deleted += other.agent_deleted;
        self.tab_added += other.tab_added;
        self.tab_deleted += other.tab_deleted;
        self.human_added += other.human_added;
        self.human_deleted += other.human_deleted;
        self.blank_added += other.blank_added;
        self.blank_deleted += other.blank_deleted;
        self.unattributed_added += other.unattributed_added;
        self.unattributed_deleted += other.unattributed_deleted;
    }

    /// Keeps whichever reading saw more of the day.
    pub(crate) fn keep_fuller(&mut self, other: &Self) {
        self.agent_added = self.agent_added.max(other.agent_added);
        self.agent_deleted = self.agent_deleted.max(other.agent_deleted);
        self.tab_added = self.tab_added.max(other.tab_added);
        self.tab_deleted = self.tab_deleted.max(other.tab_deleted);
        self.human_added = self.human_added.max(other.human_added);
        self.human_deleted = self.human_deleted.max(other.human_deleted);
        self.blank_added = self.blank_added.max(other.blank_added);
        self.blank_deleted = self.blank_deleted.max(other.blank_deleted);
        self.unattributed_added = self.unattributed_added.max(other.unattributed_added);
        self.unattributed_deleted = self.unattributed_deleted.max(other.unattributed_deleted);
    }
}

/// How much of something each author accounts for.
///
/// Used for lines and for seconds, because the question is the same shape in both
/// cases and a second type of two `u64`s would only invite them to drift apart.
///
/// This is a narrower question than `LineCounts` answers. That one describes what
/// landed in a commit and can say "nobody is sure who wrote this"; this one
/// describes what the editor watched happen, where the author is known by
/// construction. Keeping them apart means a card can show a split it is confident
/// about without borrowing the commit record's uncertainty.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AuthorShare {
    pub agent: u64,
    pub human: u64,
}

impl AuthorShare {
    pub fn total(&self) -> u64 {
        self.agent + self.human
    }

    pub fn ai_share_basis_points(&self) -> u32 {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        u32::try_from(self.agent.saturating_mul(10_000) / total).unwrap_or(10_000)
    }
}

/// Lines put in and lines taken out, kept apart.
///
/// Sources differ in what they can see. An editor watching an agent write counts
/// the lines it produced and has no notion of one removed; a commit knows both.
/// Carrying the pair everywhere lets a figure say which it is, instead of
/// presenting their sum as though it were an amount of work — a rewrite that
/// replaces a hundred lines with a hundred others is two hundred by that sum and
/// no larger than a file by any honest reading.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Lines {
    pub added: u64,
    pub deleted: u64,
}

impl Lines {
    pub fn total(&self) -> u64 {
        self.added + self.deleted
    }

    pub fn absorb(&mut self, other: &Self) {
        self.added += other.added;
        self.deleted += other.deleted;
    }
}

/// Lines by author, with additions and removals still apart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineShare {
    pub agent: Lines,
    pub human: Lines,
}

impl LineShare {
    pub fn total(&self) -> u64 {
        self.agent.total() + self.human.total()
    }

    pub fn added(&self) -> u64 {
        self.agent.added + self.human.added
    }

    pub fn deleted(&self) -> u64 {
        self.agent.deleted + self.human.deleted
    }

    pub fn ai_share_basis_points(&self) -> u32 {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        u32::try_from(self.agent.total().saturating_mul(10_000) / total).unwrap_or(10_000)
    }

    fn absorb(&mut self, author: Author, lines: &Lines) {
        match author {
            Author::Agent => self.agent.absorb(lines),
            Author::Human => self.human.absorb(lines),
        }
    }
}

/// Line facts folded up, every field a different fold over the same entries.
///
/// This is a result rather than a stored shape. Nothing writes it to a file; it
/// exists so a caller can ask one question of a span of days without walking the
/// facts itself, and so that adding a new question means adding a field here
/// rather than a field to every day on record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineTotals {
    pub by_model: BTreeMap<String, Lines>,
    pub by_language: BTreeMap<String, LineShare>,
    pub authors: LineShare,
}

impl LineTotals {
    /// Folds the facts of one day in.
    pub fn absorb_facts(&mut self, facts: &[LineFact]) {
        for fact in facts {
            let lines = Lines {
                added: fact.added,
                deleted: fact.deleted,
            };
            self.authors.absorb(fact.author, &lines);
            self.by_language
                .entry(fact.language.clone())
                .or_default()
                .absorb(fact.author, &lines);
            if fact.author == Author::Agent && !fact.model.is_empty() {
                self.by_model
                    .entry(fact.model.clone())
                    .or_default()
                    .absorb(&lines);
            }
        }
    }

    pub fn total(&self) -> u64 {
        self.authors.total()
    }

    pub fn added(&self) -> u64 {
        self.authors.added()
    }

    pub fn deleted(&self) -> u64 {
        self.authors.deleted()
    }

    pub fn ai_share_basis_points(&self) -> u32 {
        self.authors.ai_share_basis_points()
    }

    /// Models that wrote anything, largest first.
    pub fn models(&self) -> Vec<(&str, Lines)> {
        let mut ranked = self
            .by_model
            .iter()
            .map(|(model, lines)| (model.as_str(), *lines))
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .1
                .total()
                .cmp(&left.1.total())
                .then_with(|| left.0.cmp(right.0))
        });
        ranked
    }
}

/// A day as the first schema wrote it, read only so it can be brought forward.
///
/// The record accumulates over years and outlives the shape it was first written
/// in, so an old day file has to keep meaning something. This is that promise
/// kept: the fields the first schema had, and the one conversion into the fields
/// the current one has.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LegacyDay {
    pub date: String,
    pub editor: TimeBucket,
    pub agent: TimeBucket,
    pub committed: LineCounts,
    pub generated: LegacyGenerated,
    pub requests: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LegacyGenerated {
    pub by_model: BTreeMap<String, u64>,
    pub human: u64,
}

impl LegacyDay {
    /// Brings a first-schema day forward.
    ///
    /// The old shape counted an agent's lines per model and a person's lines as
    /// one number, neither of them by language, so the facts this produces carry
    /// no language. That is the truth about those days rather than a shortcoming
    /// of the conversion: the language was never recorded, and guessing one would
    /// put a figure in the record that nothing measured.
    pub fn bring_forward(self) -> DayBucket {
        let mut day = DayBucket::new(self.date);
        if self.editor != TimeBucket::default() {
            *day.measure_mut(MEASURE_EDITOR) = self.editor;
        }
        if self.agent != TimeBucket::default() {
            *day.measure_mut(MEASURE_AGENT) = self.agent;
        }
        day.commits = self.committed;
        day.requests = self.requests;
        for (model, lines) in self.generated.by_model {
            day.add_lines(UNKNOWN_LANGUAGE, Author::Agent, &model, lines, 0);
        }
        if self.generated.human > 0 {
            day.add_lines(UNKNOWN_LANGUAGE, Author::Human, "", self.generated.human, 0);
        }
        day
    }
}

/// One way of measuring time spent. Two of these live side by side in a day
/// because "the editor was in use" and "code was being changed by an agent" are
/// different quantities, and a day can be long in one and short in the other.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TimeBucket {
    pub seconds: u64,
    pub languages: BTreeMap<String, u64>,
    pub sessions: u32,
    /// The same seconds, said again at whatever grain the source knew.
    ///
    /// Not every source knows one: a tracker that watches an editor reports how
    /// long a file was worked on and has no view on who or what was doing it. So
    /// this refines `seconds` rather than replacing it, and never exceeds it.
    /// What is missing is time nobody could attribute, which is a different
    /// thing from time attributed to nobody.
    #[serde(default, alias = "byAuthor", skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<TimeFact>,
}

impl TimeBucket {
    pub(crate) fn absorb(&mut self, other: &Self) {
        self.seconds += other.seconds;
        self.sessions += other.sessions;
        for (language, seconds) in &other.languages {
            *self.languages.entry(language.clone()).or_default() += seconds;
        }
        for fact in &other.facts {
            self.spend(&fact.language, fact.author, &fact.model, fact.seconds);
        }
    }

    /// Keeps whichever reading saw more of the day, whole.
    ///
    /// Field by field would be wrong here in a way it is not wrong for counts. A
    /// reading of a day's time is one accumulation: its total, the languages it
    /// divides into and the facts refining those all come out of a single pass
    /// over a single timeline. Taking the largest of each field separately mixes
    /// two accumulations, and the parts then need not fit inside either total —
    /// a language's seconds from one reading beside a total from the other can
    /// sum to more than the day was long, which was observed happening. So the
    /// fuller reading wins entire, and every part stays with the whole it was
    /// measured against.
    pub(crate) fn keep_fuller(&mut self, other: &Self) {
        if other.seconds > self.seconds {
            *self = other.clone();
        }
    }

    /// Adds attributed seconds, keeping one entry per language, author and model.
    pub fn spend(&mut self, language: &str, author: Author, model: &str, seconds: u64) {
        self.fact_mut(language, author, model).seconds += seconds;
    }

    /// The seconds in one language that a given author is known to account for.
    pub fn attributed(&self, language: &str, author: Author) -> u64 {
        self.facts
            .iter()
            .filter(|fact| fact.language == language && fact.author == author)
            .map(|fact| fact.seconds)
            .sum()
    }

    fn fact_mut(&mut self, language: &str, author: Author, model: &str) -> &mut TimeFact {
        let key = (language, author, model);
        match self.facts.binary_search_by(|held| held.key().cmp(&key)) {
            Ok(index) => &mut self.facts[index],
            Err(index) => {
                self.facts.insert(
                    index,
                    TimeFact {
                        language: language.to_owned(),
                        author,
                        model: model.to_owned(),
                        seconds: 0,
                    },
                );
                &mut self.facts[index]
            }
        }
    }
}

/// One fact about time spent: in what language, by whom, and with which model.
///
/// Deliberately the same grain as `LineFact`, so that any breakdown the cards
/// draw of lines can also be drawn of hours without collecting anything new. The
/// rule that produces these is the same one that puts seconds against a language
/// at all: a gap between two moments belongs to the moment before it, so it
/// belongs to that moment's file, to whoever caused it, and to whatever model
/// was answering. Naming the author is therefore no more of an estimate than
/// naming the language, which is why they are recorded together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeFact {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub language: String,
    /// No default: a fact whose author is unknown is what `seconds` is for.
    pub author: Author,
    /// Empty for a person's time, and for an agent whose model went unrecorded.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(default)]
    pub seconds: u64,
}

impl TimeFact {
    /// What makes two facts the same fact. Folding and merging both key on this.
    fn key(&self) -> (&str, Author, &str) {
        (&self.language, self.author, &self.model)
    }
}

/// Time facts folded up, every field a different fold over the same entries.
///
/// The counterpart of `LineTotals`, and the reason a chart can offer hours beside
/// any figure it already draws: both are folds over facts of the same shape, so
/// asking "and how long did that take" needs no new field on any day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeTotals {
    pub by_model: BTreeMap<String, u64>,
    pub by_language: BTreeMap<String, AuthorShare>,
    pub authors: AuthorShare,
}

impl TimeTotals {
    /// Folds the facts of one measure in.
    pub fn absorb_facts(&mut self, facts: &[TimeFact]) {
        for fact in facts {
            let language = self.by_language.entry(fact.language.clone()).or_default();
            match fact.author {
                Author::Agent => {
                    self.authors.agent += fact.seconds;
                    language.agent += fact.seconds;
                    if !fact.model.is_empty() {
                        *self.by_model.entry(fact.model.clone()).or_default() += fact.seconds;
                    }
                }
                Author::Human => {
                    self.authors.human += fact.seconds;
                    language.human += fact.seconds;
                }
            }
        }
    }

    /// Seconds against one model, or nothing if that model has none.
    pub fn model(&self, name: &str) -> u64 {
        self.by_model.get(name).copied().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayBucket {
    pub date: String,
    /// Measures of time, by name. Kept as a map rather than as fields because
    /// which measures exist depends on what is installed where the work happened,
    /// and because two of them can cover the same minute — an agent changing code
    /// while its operator watches is both agent time and editor time. Nothing
    /// sums across this map.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub time: BTreeMap<String, TimeBucket>,
    /// Lines written, one entry per language, author, and model. Kept sorted with
    /// one entry per distinct triple.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<LineFact>,
    /// What landed in commits, which unlike `lines` can be honest about not
    /// knowing who wrote something.
    #[serde(default)]
    pub commits: LineCounts,
    /// Tokens by model, where a source reports them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<String, TokenUsage>,
    #[serde(default)]
    pub requests: u32,
}

impl DayBucket {
    pub fn new(date: impl Into<String>) -> Self {
        Self {
            date: date.into(),
            ..Self::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.time.values().all(|bucket| bucket.seconds == 0)
            && self.lines.is_empty()
            && self.commits.changed() == 0
            && self.tokens.is_empty()
    }

    /// The named measure, or an empty one. Reading a measure a machine never
    /// collected is an ordinary thing to do, not a mistake.
    pub fn measure(&self, name: &str) -> TimeBucket {
        self.time.get(name).cloned().unwrap_or_default()
    }

    pub fn measure_mut(&mut self, name: &str) -> &mut TimeBucket {
        self.time.entry(name.to_owned()).or_default()
    }

    /// Adds to the fact for this language, author, and model, creating it if this
    /// is the first time the triple has been seen.
    pub fn add_lines(
        &mut self,
        language: &str,
        author: Author,
        model: &str,
        added: u64,
        deleted: u64,
    ) {
        let key = (language, author, model);
        match self.lines.binary_search_by(|fact| fact.key().cmp(&key)) {
            Ok(index) => {
                self.lines[index].added += added;
                self.lines[index].deleted += deleted;
            }
            Err(index) => self.lines.insert(
                index,
                LineFact {
                    language: language.to_owned(),
                    author,
                    model: model.to_owned(),
                    added,
                    deleted,
                },
            ),
        }
    }

    pub(crate) fn absorb(&mut self, other: &Self) {
        for (name, bucket) in &other.time {
            self.measure_mut(name).absorb(bucket);
        }
        self.requests += other.requests;
        self.commits.absorb(&other.commits);
        for (model, usage) in &other.tokens {
            self.tokens.entry(model.clone()).or_default().absorb(usage);
        }
        for fact in &other.lines {
            self.add_lines(
                &fact.language,
                fact.author,
                &fact.model,
                fact.added,
                fact.deleted,
            );
        }
    }

    /// Combines two readings of the same day, keeping the fuller one field by
    /// field.
    ///
    /// This is not `absorb`. Absorbing sums, which is right for two machines or
    /// two sources contributing different work to one day, and wrong for reading
    /// the same day twice: the second reading would double it.
    ///
    /// Taking the larger of each field is right because a day in the past holds a
    /// fixed amount of work, and a reading of it can only be complete or cut
    /// short — never larger than the truth. The editor's own store keeps about a
    /// month, so a day re-read after that window reads as empty; the reading taken
    /// while it was still there is the better one and is what survives. Re-reading
    /// a day still inside the window reproduces the same numbers, so nothing
    /// changes, and re-reading today after more work reads larger, so the new
    /// figure wins.
    ///
    /// The cost of this rule is that a genuine correction downwards — a fix to how
    /// time is counted, say — cannot land on days already published, because the
    /// old larger figure outranks it. Such a change needs the day files deleted
    /// rather than rewritten.
    pub fn keep_fuller(&mut self, other: &Self) {
        for (name, bucket) in &other.time {
            self.measure_mut(name).keep_fuller(bucket);
        }
        self.requests = self.requests.max(other.requests);
        self.commits.keep_fuller(&other.commits);
        for (model, usage) in &other.tokens {
            self.tokens
                .entry(model.clone())
                .or_default()
                .keep_fuller(usage);
        }
        for fact in &other.lines {
            let key = fact.key();
            match self.lines.binary_search_by(|held| held.key().cmp(&key)) {
                Ok(index) => {
                    self.lines[index].added = self.lines[index].added.max(fact.added);
                    self.lines[index].deleted = self.lines[index].deleted.max(fact.deleted);
                }
                Err(index) => self.lines.insert(index, fact.clone()),
            }
        }
        self.settle_unknown_language();
    }

    /// Drops the part of an unnamed language that a named one already accounts for.
    ///
    /// Two readings of the same day can describe the same lines at different
    /// grains: a day recorded before languages were kept says only that a model
    /// wrote so many lines, and a later reading of the same day says which
    /// languages they were. Both survive the merge because they are filed under
    /// different keys, and the day would then count those lines twice.
    ///
    /// The coarse reading is not wrong, only less specific, so what it keeps is
    /// the remainder: whatever it claimed beyond what the named languages
    /// explain. Usually that is nothing and the row disappears; where the finer
    /// reading is incomplete, the difference stays under an unnamed language
    /// rather than being discarded.
    fn settle_unknown_language(&mut self) {
        let mut named: BTreeMap<(Author, String), (u64, u64)> = BTreeMap::new();
        for fact in &self.lines {
            if fact.language == UNKNOWN_LANGUAGE {
                continue;
            }
            let explained = named
                .entry((fact.author, fact.model.clone()))
                .or_insert((0, 0));
            explained.0 += fact.added;
            explained.1 += fact.deleted;
        }

        for fact in &mut self.lines {
            if fact.language != UNKNOWN_LANGUAGE {
                continue;
            }
            if let Some((added, deleted)) = named.get(&(fact.author, fact.model.clone())) {
                fact.added = fact.added.saturating_sub(*added);
                fact.deleted = fact.deleted.saturating_sub(*deleted);
            }
        }
        self.lines
            .retain(|fact| fact.language != UNKNOWN_LANGUAGE || fact.total() > 0);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySnapshot {
    pub schema: u32,
    pub machine: String,
    pub collected_at: String,
    #[serde(default)]
    pub cursors: BTreeMap<String, String>,
    #[serde(default)]
    pub days: Vec<DayBucket>,
}

impl ActivitySnapshot {
    pub fn new(machine: impl Into<String>, collected_at: impl Into<String>) -> Self {
        Self {
            schema: ACTIVITY_SCHEMA,
            machine: machine.into(),
            collected_at: collected_at.into(),
            cursors: BTreeMap::new(),
            days: Vec::new(),
        }
    }

    /// Whether this holds the same record as another, disregarding when it was
    /// collected. Collecting again moves `collected_at` whether or not anything
    /// happened, so a sink that keeps history needs a way to tell a real change
    /// from a clock reading.
    pub fn records_the_same_as(&self, other: &Self) -> bool {
        self.schema == other.schema && self.machine == other.machine && self.days == other.days
    }

    pub fn validate(&self) -> Result<(), GithubStatsError> {
        if self.schema > ACTIVITY_SCHEMA {
            return Err(GithubStatsError::InvalidResponse {
                message: format!(
                    "snapshot schema {} is newer than this build understands ({ACTIVITY_SCHEMA})",
                    self.schema
                ),
            });
        }
        validate_machine(&self.machine)?;
        validate_timestamp(&self.collected_at)?;
        for day in &self.days {
            validate_date(&day.date)?;
        }
        Ok(())
    }
}

pub fn parse_activity_snapshot(input: &str) -> Result<ActivitySnapshot, GithubStatsError> {
    let snapshot = serde_json::from_str::<ActivitySnapshot>(input).map_err(|error| {
        GithubStatsError::InvalidResponse {
            message: format!("could not read activity snapshot: {error}"),
        }
    })?;
    snapshot.validate()?;
    Ok(snapshot)
}

pub fn write_activity_snapshot(snapshot: &ActivitySnapshot) -> Result<String, GithubStatsError> {
    snapshot.validate()?;
    let mut ordered = snapshot.clone();
    ordered
        .days
        .sort_by(|left, right| left.date.cmp(&right.date));
    serde_json::to_string_pretty(&ordered)
        .map(|text| format!("{text}\n"))
        .map_err(|error| GithubStatsError::InvalidResponse {
            message: format!("could not write activity snapshot: {error}"),
        })
}

pub fn merge_snapshots(snapshots: &[ActivitySnapshot]) -> Vec<DayBucket> {
    let mut newest = BTreeMap::<(&str, &str), (&str, &DayBucket)>::new();

    for snapshot in snapshots {
        for day in &snapshot.days {
            let key = (snapshot.machine.as_str(), day.date.as_str());
            match newest.get(&key) {
                Some((seen_at, _)) if *seen_at >= snapshot.collected_at.as_str() => {}
                _ => {
                    newest.insert(key, (snapshot.collected_at.as_str(), day));
                }
            }
        }
    }

    let mut totals = BTreeMap::<&str, DayBucket>::new();
    for ((_, date), (_, day)) in newest {
        totals
            .entry(date)
            .or_insert_with(|| DayBucket::new(date))
            .absorb(day);
    }

    totals.into_values().collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelUsage {
    pub name: String,
    pub lines: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MeasureTotals {
    pub seconds: u64,
    pub sessions: u32,
    pub languages: Vec<CodingActivityEntry>,
    pub daily_seconds: Vec<(String, u64)>,
    /// The part of `seconds` a source could put a name to, folded every way.
    pub facts: TimeTotals,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityTotals {
    pub first_day: Option<String>,
    pub last_day: Option<String>,
    pub active_days: u32,
    /// Every measure the days carried, by name. A reader shows the ones it finds
    /// rather than the ones it was built expecting.
    pub time: BTreeMap<String, MeasureTotals>,
    pub commits: LineCounts,
    pub lines: LineTotals,
    pub tokens: BTreeMap<String, TokenUsage>,
    pub requests: u32,
}

impl ActivityTotals {
    pub fn measure(&self, name: &str) -> MeasureTotals {
        self.time.get(name).cloned().unwrap_or_default()
    }

    /// Measures that recorded anything, longest first.
    pub fn measures(&self) -> Vec<(&str, &MeasureTotals)> {
        let mut ranked = self
            .time
            .iter()
            .filter(|(_, totals)| totals.seconds > 0)
            .map(|(name, totals)| (name.as_str(), totals))
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .1
                .seconds
                .cmp(&left.1.seconds)
                .then_with(|| left.0.cmp(right.0))
        });
        ranked
    }

    pub fn models(&self) -> Vec<ModelUsage> {
        self.lines
            .models()
            .into_iter()
            .map(|(name, lines)| ModelUsage {
                name: name.to_owned(),
                lines: lines.total(),
            })
            .collect()
    }

    pub fn tokens_billed(&self) -> u64 {
        self.tokens.values().map(TokenUsage::billed).sum()
    }
}

pub fn summarise_activity(days: &[DayBucket]) -> ActivityTotals {
    let mut languages = BTreeMap::<String, BTreeMap<String, u64>>::new();
    let mut totals = ActivityTotals::default();

    for day in days {
        if !day.is_empty() {
            totals.active_days += 1;
        }
        totals.requests += day.requests;
        totals.commits.absorb(&day.commits);
        totals.lines.absorb_facts(&day.lines);
        for (model, usage) in &day.tokens {
            totals
                .tokens
                .entry(model.clone())
                .or_default()
                .absorb(usage);
        }
        for (name, bucket) in &day.time {
            let measure = totals.time.entry(name.clone()).or_default();
            measure.seconds += bucket.seconds;
            measure.sessions += bucket.sessions;
            measure.facts.absorb_facts(&bucket.facts);
            measure
                .daily_seconds
                .push((day.date.clone(), bucket.seconds));
            let held = languages.entry(name.clone()).or_default();
            for (language, seconds) in &bucket.languages {
                *held.entry(language.clone()).or_default() += seconds;
            }
        }
    }

    totals.first_day = days.first().map(|day| day.date.clone());
    totals.last_day = days.last().map(|day| day.date.clone());
    for (name, counts) in languages {
        if let Some(measure) = totals.time.get_mut(&name) {
            measure.languages = rank(
                counts
                    .iter()
                    .map(|(language, seconds)| (language.as_str(), *seconds))
                    .collect(),
            )
            .into_iter()
            .map(|(language, seconds)| CodingActivityEntry {
                language: language.to_owned(),
                seconds,
            })
            .collect();
        }
    }
    totals
}

/// Largest first, ties broken by name so the order is the same on every run.
fn rank(counts: Vec<(&str, u64)>) -> Vec<(&str, u64)> {
    let mut ranked = counts;
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    ranked
}

pub(crate) fn validate_machine(machine: &str) -> Result<(), GithubStatsError> {
    let usable = !machine.is_empty()
        && machine.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        });
    if usable {
        return Ok(());
    }
    Err(GithubStatsError::InvalidResponse {
        message: format!(
            "machine id {machine:?} must be lowercase letters, digits, and dashes so it can name a file"
        ),
    })
}

pub(crate) fn validate_timestamp(value: &str) -> Result<(), GithubStatsError> {
    if value.len() == TIMESTAMP_LENGTH && value.ends_with('Z') {
        return Ok(());
    }
    Err(GithubStatsError::InvalidResponse {
        message: format!("collected_at {value:?} must look like 2026-08-24T19:00:00Z"),
    })
}

pub(crate) fn validate_date(value: &str) -> Result<(), GithubStatsError> {
    let shaped = value.len() == DATE_LENGTH
        && value
            .chars()
            .enumerate()
            .all(|(index, character)| match index {
                4 | 7 => character == '-',
                _ => character.is_ascii_digit(),
            });
    if shaped {
        return Ok(());
    }
    Err(GithubStatsError::InvalidResponse {
        message: format!("day {value:?} must look like 2026-08-24"),
    })
}
