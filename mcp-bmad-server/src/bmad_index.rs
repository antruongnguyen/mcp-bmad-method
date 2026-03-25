use std::collections::HashMap;

/// Raw BMad Method documentation embedded at compile time.
static BMAD_DOCS: &str = include_str!("../data/llms-full.txt");

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// The four phases of the BMad Method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Analysis,
    Planning,
    Solutioning,
    Implementation,
}

#[allow(dead_code)]
impl Phase {
    pub fn all() -> &'static [Phase] {
        &[
            Phase::Analysis,
            Phase::Planning,
            Phase::Solutioning,
            Phase::Implementation,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Phase::Analysis => "Analysis",
            Phase::Planning => "Planning",
            Phase::Solutioning => "Solutioning",
            Phase::Implementation => "Implementation",
        }
    }

    pub fn number(&self) -> u8 {
        match self {
            Phase::Analysis => 1,
            Phase::Planning => 2,
            Phase::Solutioning => 3,
            Phase::Implementation => 4,
        }
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, Phase::Analysis)
    }

    pub fn description(&self) -> &'static str {
        match self {
            Phase::Analysis => {
                "Explore the problem space and validate ideas before committing to planning."
            }
            Phase::Planning => "Define what to build and for whom.",
            Phase::Solutioning => "Decide how to build it and break work into stories.",
            Phase::Implementation => "Build it, one story at a time.",
        }
    }
}

/// The three planning tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Track {
    QuickFlow,
    BmadMethod,
    Enterprise,
}

#[allow(dead_code)]
impl Track {
    pub fn all() -> &'static [Track] {
        &[Track::QuickFlow, Track::BmadMethod, Track::Enterprise]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Track::QuickFlow => "Quick Flow",
            Track::BmadMethod => "BMad Method",
            Track::Enterprise => "Enterprise",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Track::QuickFlow => {
                "Bug fixes, simple features, clear scope (1-15 stories). Tech-spec only."
            }
            Track::BmadMethod => {
                "Products, platforms, complex features (10-50+ stories). PRD + Architecture + UX."
            }
            Track::Enterprise => "Compliance, multi-tenant systems (30+ stories). PRD + Architecture + Security + DevOps.",
        }
    }

    /// Which phases apply to this track.
    pub fn phases(&self) -> Vec<Phase> {
        match self {
            Track::QuickFlow => vec![Phase::Implementation],
            Track::BmadMethod => vec![
                Phase::Analysis,
                Phase::Planning,
                Phase::Solutioning,
                Phase::Implementation,
            ],
            Track::Enterprise => vec![
                Phase::Analysis,
                Phase::Planning,
                Phase::Solutioning,
                Phase::Implementation,
            ],
        }
    }
}

/// An agent in the BMad Method.
#[derive(Debug, Clone)]
pub struct Agent {
    pub name: &'static str,
    pub persona: &'static str,
    pub skill_id: &'static str,
    pub primary_workflows: Vec<&'static str>,
}

/// A workflow (skill/command) in the BMad Method.
#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: &'static str,
    pub description: &'static str,
    pub phase: Phase,
    pub agent: &'static str,
    pub produces: &'static str,
    pub prerequisites: Vec<&'static str>,
    pub next_steps: Vec<&'static str>,
    /// Which tracks this workflow applies to.
    pub tracks: Vec<Track>,
}

/// A core tool (always available, not tied to a workflow phase).
#[derive(Debug, Clone)]
pub struct CoreTool {
    pub id: &'static str,
    pub description: &'static str,
}

// ---------------------------------------------------------------------------
// Index
// ---------------------------------------------------------------------------

/// Result of a readiness check for entering Implementation phase.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ReadinessResult {
    pub ready: bool,
    pub missing_artifacts: Vec<String>,
    pub warnings: Vec<String>,
    pub next_action: String,
}

/// Result from scanning a project directory for BMad artifacts.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProjectStateResult {
    pub bmad_installed: bool,
    pub prd_found: bool,
    pub architecture_found: bool,
    pub epic_count: usize,
    pub sprint_status_found: bool,
    pub project_context_found: bool,
}

/// Result from scaffolding a BMad project.
#[derive(Debug, Clone)]
pub struct ScaffoldResult {
    pub files_created: Vec<String>,
    pub track: Track,
    pub next_steps: Vec<&'static str>,
}

/// Result from sprint guide cycle detection.
pub struct SprintGuideResult {
    pub current_step: &'static str,
    pub agent_to_invoke: &'static str,
    pub workflow_to_run: &'static str,
    pub rationale: &'static str,
    pub after_this: &'static str,
}

/// Describes where the index docs came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocSource {
    Embedded,
    Url(String),
    Cache(String),
}

impl std::fmt::Display for DocSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocSource::Embedded => write!(f, "embedded"),
            DocSource::Url(url) => write!(f, "url: {url}"),
            DocSource::Cache(path) => write!(f, "cache: {path}"),
        }
    }
}

/// Result of validating doc content before index rebuild.
#[derive(Debug)]
pub struct DocValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

/// The in-memory index of all BMad Method content.
pub struct BmadIndex {
    workflows: HashMap<&'static str, Workflow>,
    agents: HashMap<&'static str, Agent>,
    core_tools: Vec<CoreTool>,
    phase_workflows: HashMap<Phase, Vec<&'static str>>,
    docs: String,
    doc_source: DocSource,
    last_refresh: Option<std::time::Instant>,
    doc_byte_size: usize,
}

#[allow(dead_code)]
impl BmadIndex {
    /// Build the index with the embedded documentation.
    pub fn build() -> Self {
        Self::build_with_source(BMAD_DOCS.to_string(), DocSource::Embedded)
    }

    /// Build the index with custom documentation content.
    pub fn build_with_docs(docs: String) -> Self {
        Self::build_with_source(docs, DocSource::Embedded)
    }

    /// Build the index with custom documentation content and source metadata.
    pub fn build_with_source(docs: String, source: DocSource) -> Self {
        let agents = Self::build_agents();
        let workflows = Self::build_workflows();
        let core_tools = Self::build_core_tools();

        let mut phase_workflows: HashMap<Phase, Vec<&'static str>> = HashMap::new();
        for phase in Phase::all() {
            phase_workflows.insert(*phase, Vec::new());
        }
        for (id, wf) in &workflows {
            phase_workflows.entry(wf.phase).or_default().push(id);
        }

        let doc_byte_size = docs.len();

        Self {
            workflows,
            agents,
            core_tools,
            phase_workflows,
            docs,
            doc_source: source,
            last_refresh: Some(std::time::Instant::now()),
            doc_byte_size,
        }
    }

    /// Validate that documentation content has the minimum expected structure.
    ///
    /// Checks for at least the 4 phase names and a few known workflow ids.
    pub fn validate_docs(docs: &str) -> DocValidationResult {
        let mut errors = Vec::new();
        let lower = docs.to_lowercase();

        // Check for the 4 phase names
        const EXPECTED_PHASES: &[&str] = &["analysis", "planning", "solutioning", "implementation"];
        for phase in EXPECTED_PHASES {
            if !lower.contains(phase) {
                errors.push(format!("missing expected phase name: {phase}"));
            }
        }

        // Check for a few known workflow-related keywords
        const EXPECTED_KEYWORDS: &[&str] = &["prd", "architecture", "sprint", "epic"];
        for kw in EXPECTED_KEYWORDS {
            if !lower.contains(kw) {
                errors.push(format!("missing expected keyword: {kw}"));
            }
        }

        // Minimum size check — valid docs should be substantial
        if docs.len() < 500 {
            errors.push(format!(
                "document too small ({} bytes, expected at least 500)",
                docs.len()
            ));
        }

        DocValidationResult {
            valid: errors.is_empty(),
            errors,
        }
    }

    /// Return the doc source metadata.
    pub fn doc_source(&self) -> &DocSource {
        &self.doc_source
    }

    /// Return the last refresh instant, if any.
    pub fn last_refresh(&self) -> Option<std::time::Instant> {
        self.last_refresh
    }

    /// Return the doc byte size.
    pub fn doc_byte_size(&self) -> usize {
        self.doc_byte_size
    }

    /// Return the raw documentation (embedded or custom).
    pub fn raw_docs(&self) -> &str {
        &self.docs
    }

    /// Return the embedded documentation (always available).
    pub fn embedded_docs() -> &'static str {
        BMAD_DOCS
    }

    // -- Workflow queries --

    /// Look up a single workflow by its skill id (e.g. `"bmad-create-prd"`).
    pub fn get_workflow(&self, id: &str) -> Option<&Workflow> {
        self.workflows.get(id)
    }

    /// All workflow ids in a given phase, in recommended order.
    pub fn get_phase_workflows(&self, phase: Phase) -> &[&'static str] {
        self.phase_workflows
            .get(&phase)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Return the ordered list of next-step workflow ids after completing
    /// all workflows in `phase`.
    pub fn get_next_steps(&self, phase: Phase) -> Vec<&'static str> {
        match phase {
            Phase::Analysis => vec!["bmad-create-prd", "bmad-quick-dev"],
            Phase::Planning => vec![
                "bmad-create-architecture",
                "bmad-create-ux-design",
                "bmad-quick-dev",
            ],
            Phase::Solutioning => vec!["bmad-sprint-planning"],
            Phase::Implementation => vec!["bmad-retrospective"],
        }
    }

    /// Return the next-step workflow ids after completing a specific workflow.
    pub fn get_workflow_next_steps(&self, id: &str) -> Vec<&'static str> {
        self.workflows
            .get(id)
            .map(|w| w.next_steps.clone())
            .unwrap_or_default()
    }

    /// Return all workflows that belong to a given track.
    pub fn get_track_workflows(&self, track: Track) -> Vec<&Workflow> {
        self.workflows
            .values()
            .filter(|w| w.tracks.contains(&track))
            .collect()
    }

    /// List all workflow ids.
    pub fn all_workflow_ids(&self) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = self.workflows.keys().copied().collect();
        ids.sort();
        ids
    }

    // -- Agent queries --

    /// Look up an agent by skill id (e.g. `"bmad-dev"`).
    pub fn get_agent(&self, skill_id: &str) -> Option<&Agent> {
        self.agents.get(skill_id)
    }

    /// List all agents.
    pub fn all_agents(&self) -> Vec<&Agent> {
        let mut agents: Vec<&Agent> = self.agents.values().collect();
        agents.sort_by_key(|a| a.skill_id);
        agents
    }

    /// Return agents whose primary workflows include at least one workflow in
    /// the given phase.
    pub fn get_agents_by_phase(&self, phase: Phase) -> Vec<&Agent> {
        let phase_wfs = self.get_phase_workflows(phase);
        let mut agents: Vec<&Agent> = self
            .agents
            .values()
            .filter(|a| {
                a.primary_workflows
                    .iter()
                    .any(|wf_id| phase_wfs.contains(wf_id))
            })
            .collect();
        agents.sort_by_key(|a| a.skill_id);
        agents
    }

    // -- Core tools --

    pub fn core_tools(&self) -> &[CoreTool] {
        &self.core_tools
    }

    // -- State inference --

    /// Known artifact keywords and the workflow that produces them.
    const ARTIFACT_MAP: &[(&str, &str)] = &[
        ("brainstorming report", "bmad-brainstorming"),
        ("brainstorming-report", "bmad-brainstorming"),
        ("product brief", "bmad-create-product-brief"),
        ("product-brief", "bmad-create-product-brief"),
        ("prd", "bmad-create-prd"),
        ("ux spec", "bmad-create-ux-design"),
        ("ux-spec", "bmad-create-ux-design"),
        ("ux design", "bmad-create-ux-design"),
        ("architecture", "bmad-create-architecture"),
        ("architecture.md", "bmad-create-architecture"),
        ("adr", "bmad-create-architecture"),
        ("epics", "bmad-create-epics-and-stories"),
        ("stories", "bmad-create-epics-and-stories"),
        ("epic", "bmad-create-epics-and-stories"),
        ("story", "bmad-create-epics-and-stories"),
        ("readiness check", "bmad-check-implementation-readiness"),
        ("implementation readiness", "bmad-check-implementation-readiness"),
        ("sprint status", "bmad-sprint-planning"),
        ("sprint-status", "bmad-sprint-planning"),
        ("sprint plan", "bmad-sprint-planning"),
    ];

    /// Parse a free-text project state description and return the set of
    /// workflow ids whose artifacts are mentioned as existing.
    pub fn infer_completed_workflows(project_state: &str) -> Vec<&'static str> {
        let lower = project_state.to_lowercase();
        let mut completed = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for &(keyword, workflow_id) in Self::ARTIFACT_MAP {
            if lower.contains(keyword) && seen.insert(workflow_id) {
                completed.push(workflow_id);
            }
        }

        completed
    }

    /// Determine which phase the project is currently in, based on which
    /// workflows have been completed.
    pub fn infer_current_phase(completed: &[&str]) -> Phase {
        let has = |id: &str| completed.contains(&id);

        if has("bmad-sprint-planning") || has("bmad-check-implementation-readiness") {
            return Phase::Implementation;
        }
        if has("bmad-create-architecture") || has("bmad-create-epics-and-stories") {
            return Phase::Solutioning;
        }
        if has("bmad-create-prd") || has("bmad-create-ux-design") {
            return Phase::Planning;
        }
        if has("bmad-brainstorming")
            || has("bmad-market-research")
            || has("bmad-domain-research")
            || has("bmad-technical-research")
            || has("bmad-create-product-brief")
        {
            return Phase::Analysis;
        }
        // Nothing completed — start at Analysis
        Phase::Analysis
    }

    /// Recommend the next workflow to run given completed workflows and
    /// an optional last-completed workflow id.
    pub fn recommend_next(
        &self,
        completed: &[&str],
        last_workflow: Option<&str>,
    ) -> Vec<&Workflow> {
        // If a specific last workflow is given, use its next_steps
        if let Some(last) = last_workflow
            && let Some(wf) = self.get_workflow(last)
        {
            let candidates: Vec<&Workflow> = wf
                .next_steps
                .iter()
                .filter(|id| !completed.contains(id))
                .filter_map(|id| self.get_workflow(id))
                .collect();
            if !candidates.is_empty() {
                return candidates;
            }
        }

        // Otherwise, infer from current phase
        let phase = Self::infer_current_phase(completed);
        let next_ids = self.get_next_steps(phase);
        let candidates: Vec<&Workflow> = next_ids
            .iter()
            .filter(|id| !completed.contains(id))
            .filter_map(|id| self.get_workflow(id))
            .collect();

        if !candidates.is_empty() {
            return candidates;
        }

        // Current phase workflows that haven't been completed
        self.get_phase_workflows(phase)
            .iter()
            .filter(|id| !completed.contains(id))
            .filter_map(|id| self.get_workflow(id))
            .collect()
    }

    // -- Readiness check --

    /// Check whether a project is ready to enter the Implementation phase
    /// for the given track, based on a free-text `project_state` description.
    pub fn check_readiness(project_state: &str, track: Track) -> ReadinessResult {
        let lower = project_state.to_lowercase();

        let has = |keywords: &[&str]| -> bool {
            keywords
                .iter()
                .any(|kw| lower.contains(&kw.to_lowercase()))
        };

        let has_prd = has(&["prd", "product requirements"]);
        let has_architecture = has(&["architecture", "adr"]);
        let has_epics = has(&["epics", "epic", "stories", "story"]);
        let has_tech_spec = has(&["tech-spec", "tech spec", "spec-", "spec."]);
        let has_security = has(&["security"]);
        let has_devops = has(&["devops", "dev-ops", "infrastructure", "ci/cd", "cicd"]);

        type ArtifactCheck<'a> = Vec<(&'a str, bool)>;
        let (required, optional): (ArtifactCheck<'_>, ArtifactCheck<'_>) = match track {
            Track::QuickFlow => (
                vec![("tech-spec", has_tech_spec)],
                vec![],
            ),
            Track::BmadMethod => (
                vec![
                    ("PRD", has_prd),
                    ("architecture", has_architecture),
                    ("epics/stories", has_epics),
                ],
                vec![
                    ("UX spec", has(&["ux spec", "ux-spec", "ux design"])),
                    (
                        "implementation readiness check",
                        has(&["readiness check", "implementation readiness"]),
                    ),
                ],
            ),
            Track::Enterprise => (
                vec![
                    ("PRD", has_prd),
                    ("architecture", has_architecture),
                    ("security docs", has_security),
                    ("DevOps docs", has_devops),
                    ("epics/stories", has_epics),
                ],
                vec![
                    ("UX spec", has(&["ux spec", "ux-spec", "ux design"])),
                    (
                        "implementation readiness check",
                        has(&["readiness check", "implementation readiness"]),
                    ),
                ],
            ),
        };

        let missing: Vec<String> = required
            .iter()
            .filter(|(_, present)| !present)
            .map(|(name, _)| (*name).to_string())
            .collect();

        let warnings: Vec<String> = optional
            .iter()
            .filter(|(_, present)| !present)
            .map(|(name, _)| format!("Recommended: {name}"))
            .collect();

        let ready = missing.is_empty();

        let next_action = if ready {
            match track {
                Track::QuickFlow => {
                    "Ready! Run `bmad-quick-dev` to start implementation.".to_string()
                }
                _ => "Ready! Run `bmad-sprint-planning` to initialize the sprint and begin implementation.".to_string(),
            }
        } else {
            let first_missing = &missing[0];
            match first_missing.as_str() {
                "tech-spec" => {
                    "Create a tech-spec using `bmad-quick-dev` (Quick Flow track).".to_string()
                }
                "PRD" => "Create a PRD using `bmad-create-prd` (agent: bmad-pm).".to_string(),
                "architecture" => {
                    "Create architecture docs using `bmad-create-architecture` (agent: bmad-architect).".to_string()
                }
                "security docs" => {
                    "Add security documentation as part of the architecture phase.".to_string()
                }
                "DevOps docs" => {
                    "Add DevOps/infrastructure documentation as part of the architecture phase."
                        .to_string()
                }
                "epics/stories" => {
                    "Break requirements into epics and stories using `bmad-create-epics-and-stories` (agent: bmad-pm).".to_string()
                }
                _ => format!("Create the missing artifact: {first_missing}"),
            }
        };

        ReadinessResult {
            ready,
            missing_artifacts: missing,
            warnings,
            next_action,
        }
    }

    /// Determine the current step in the BMad build cycle from a free-text sprint state.
    ///
    /// The cycle is: SM creates story → DEV implements → DEV reviews → repeat.
    /// After all stories in an epic, SM runs retrospective. Then move to next epic.
    pub fn sprint_guide(sprint_state: &str) -> SprintGuideResult {
        let state = sprint_state.to_lowercase();

        // Simple keyword presence check.
        let has = |keywords: &[&str]| -> bool { keywords.iter().any(|kw| state.contains(kw)) };

        // Check if a keyword is present AND not preceded by a negation word.
        // E.g. "not yet implemented" should NOT count as affirming "implemented".
        let has_affirmed = |keywords: &[&str]| -> bool {
            const NEGATIONS: &[&str] = &["not ", "not yet ", "no ", "hasn't ", "hasn't been ", "without ", "isn't "];
            keywords.iter().any(|kw| {
                if let Some(pos) = state.find(kw) {
                    let prefix = &state[..pos];
                    !NEGATIONS.iter().any(|neg| prefix.ends_with(neg))
                } else {
                    false
                }
            })
        };

        if has(&[
            "no sprint",
            "no plan",
            "not started",
            "beginning",
            "brand new",
            "just started implementation",
        ]) {
            SprintGuideResult {
                current_step: "Sprint initialization",
                agent_to_invoke: "bmad-agent-sm",
                workflow_to_run: "bmad-sprint-planning",
                rationale: "No sprint plan detected. The Scrum Master needs to initialize sprint \
                    tracking (sprint-status.yaml) before stories can be created.",
                after_this: "Once sprint planning is complete, the SM will create the first story \
                    file for the first epic.",
            }
        } else if has(&[
            "all stories in epic done",
            "all stories done",
            "all stories complete",
            "epic complete",
            "epic done",
            "epic finished",
            "stories in epic complete",
            "all stories reviewed",
            "last story reviewed",
            "last story done",
            "entire epic implemented and reviewed",
        ]) {
            SprintGuideResult {
                current_step: "Epic retrospective",
                agent_to_invoke: "bmad-agent-sm",
                workflow_to_run: "bmad-retrospective",
                rationale: "All stories in the current epic are complete. The Scrum Master should \
                    run a retrospective to review what went well, what didn't, and capture \
                    lessons before moving to the next epic.",
                after_this: "After the retrospective, if more epics remain, the SM will create \
                    the first story file for the next epic. If all epics are done, the project \
                    is complete.",
            }
        } else if has_affirmed(&[
            "implemented",
            "built",
            "coded",
            "developed",
            "implementation done",
            "implementation complete",
            "code complete",
            "code done",
        ]) && !has_affirmed(&[
            "reviewed",
            "review done",
            "review complete",
            "passed review",
            "code review done",
            "code review complete",
        ]) {
            SprintGuideResult {
                current_step: "Code review",
                agent_to_invoke: "bmad-agent-dev",
                workflow_to_run: "bmad-code-review",
                rationale: "The story has been implemented but not yet reviewed. The Developer \
                    should run the code review workflow to validate the implementation quality, \
                    check for edge cases, and ensure the story acceptance criteria are met.",
                after_this: "After the code review passes, if more stories remain in the current \
                    epic, the SM will create the next story file. If this was the last story in \
                    the epic, the SM will run a retrospective.",
            }
        } else if has(&[
            "story file created",
            "story created",
            "story file exists",
            "story ready",
            "story prepared",
            "story written",
            "has story file",
            "story file done",
            "story defined",
        ]) && !has_affirmed(&[
            "implemented",
            "built",
            "coded",
            "developed",
            "implementation done",
            "code complete",
        ]) {
            SprintGuideResult {
                current_step: "Story implementation",
                agent_to_invoke: "bmad-agent-dev",
                workflow_to_run: "bmad-dev-story",
                rationale: "A story file has been created but the story has not been implemented \
                    yet. The Developer should implement the story according to its acceptance \
                    criteria and technical requirements.",
                after_this: "After implementation, the Developer will run a code review on the \
                    completed story.",
            }
        } else if has_affirmed(&[
            "reviewed",
            "review done",
            "review complete",
            "passed review",
            "code review done",
            "code review complete",
        ]) && has(&[
            "more stories",
            "stories remain",
            "next story",
            "remaining stories",
            "not all stories",
        ]) {
            SprintGuideResult {
                current_step: "Create next story",
                agent_to_invoke: "bmad-agent-sm",
                workflow_to_run: "bmad-create-story",
                rationale: "The current story has been reviewed and more stories remain in the \
                    epic. The Scrum Master should create the next story file to continue the \
                    cycle.",
                after_this: "After the story file is created, the Developer will implement it, \
                    then review it. This cycle continues until all stories in the epic are done.",
            }
        } else if has(&[
            "no story",
            "need story",
            "no current story",
            "story not created",
            "need to create story",
            "waiting for story",
        ]) {
            SprintGuideResult {
                current_step: "Create story",
                agent_to_invoke: "bmad-agent-sm",
                workflow_to_run: "bmad-create-story",
                rationale: "No current story file exists. The Scrum Master needs to create the \
                    next story file from the epic's story list so the Developer can implement it.",
                after_this: "After the story file is created, the Developer will implement the \
                    story, then run a code review.",
            }
        } else if has(&[
            "retrospective done",
            "retro done",
            "retro complete",
            "retrospective complete",
        ]) {
            SprintGuideResult {
                current_step: "Start next epic",
                agent_to_invoke: "bmad-agent-sm",
                workflow_to_run: "bmad-create-story",
                rationale: "The retrospective for the previous epic is complete. The Scrum Master \
                    should create the first story file for the next epic to begin the build \
                    cycle again.",
                after_this: "After the story file is created, the Developer will implement it. \
                    The SM creates story -> DEV implements -> DEV reviews cycle continues for \
                    each story in the new epic.",
            }
        } else if has(&["sprint plan", "sprint status", "sprint-status"]) {
            SprintGuideResult {
                current_step: "Create story",
                agent_to_invoke: "bmad-agent-sm",
                workflow_to_run: "bmad-create-story",
                rationale: "A sprint plan exists. The Scrum Master should create the next story \
                    file so the Developer can begin implementation.",
                after_this: "After the story file is created, the Developer will implement it, \
                    then run a code review.",
            }
        } else {
            SprintGuideResult {
                current_step: "Sprint initialization",
                agent_to_invoke: "bmad-agent-sm",
                workflow_to_run: "bmad-sprint-planning",
                rationale: "Could not determine the exact cycle state from the description \
                    provided. Starting from the beginning: the Scrum Master should initialize \
                    sprint tracking. Provide more detail about your sprint state for more \
                    specific guidance.",
                after_this: "Once sprint planning is complete, the SM will create story files \
                    and the DEV will implement and review them in sequence.",
            }
        }
    }

    /// Scan a project directory and return which BMad artifacts exist.
    ///
    /// Checks for the standard BMad output structure:
    /// - `_bmad/` config directory
    /// - `_bmad-output/planning-artifacts/PRD.md`
    /// - `_bmad-output/planning-artifacts/architecture.md`
    /// - `_bmad-output/planning-artifacts/epics/` (counts `.md` files)
    /// - `_bmad-output/implementation-artifacts/sprint-status.yaml`
    /// - `_bmad-output/project-context.md`
    ///
    /// Does not follow symlinks that leave `project_path`.
    pub fn scan_project_dir(project_path: &std::path::Path) -> std::io::Result<ProjectStateResult> {
        use std::fs;

        let canon = project_path.canonicalize()?;

        let bmad_installed = canon.join("_bmad").is_dir();

        let planning = canon.join("_bmad-output/planning-artifacts");
        let prd_found = planning.join("PRD.md").is_file();
        let architecture_found = planning.join("architecture.md").is_file();

        let epics_dir = planning.join("epics");
        let epic_count = if epics_dir.is_dir() {
            fs::read_dir(&epics_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let path = e.path();
                    // Only count .md files, skip symlinks that escape the project
                    path.extension().is_some_and(|ext| ext == "md")
                        && path
                            .canonicalize()
                            .ok()
                            .is_some_and(|p| p.starts_with(&canon))
                })
                .count()
        } else {
            0
        };

        let impl_dir = canon.join("_bmad-output/implementation-artifacts");
        let sprint_status_found = impl_dir.join("sprint-status.yaml").is_file();

        let project_context_found = canon.join("_bmad-output/project-context.md").is_file();

        Ok(ProjectStateResult {
            bmad_installed,
            prd_found,
            architecture_found,
            epic_count,
            sprint_status_found,
            project_context_found,
        })
    }

    /// Generate starter BMad Method files in `project_path` for the given track.
    ///
    /// Creates the standard `_bmad/` config directory and appropriate planning
    /// artifact stubs pre-filled with track-appropriate boilerplate and TODO
    /// markers. Returns the list of files created and recommended next steps.
    pub fn scaffold_project(
        project_path: &std::path::Path,
        track: Track,
    ) -> std::io::Result<ScaffoldResult> {
        use std::fs;

        let bmad_dir = project_path.join("_bmad");
        let output_dir = project_path.join("_bmad-output");
        let planning_dir = output_dir.join("planning-artifacts");
        let epics_dir = planning_dir.join("epics");
        let impl_dir = output_dir.join("implementation-artifacts");

        // Create directory structure
        fs::create_dir_all(&bmad_dir)?;
        fs::create_dir_all(&planning_dir)?;
        fs::create_dir_all(&epics_dir)?;
        fs::create_dir_all(&impl_dir)?;

        let mut files_created = Vec::new();

        // Project context (all tracks)
        let context_path = output_dir.join("project-context.md");
        fs::write(
            &context_path,
            Self::template_project_context(track),
        )?;
        files_created.push(Self::relative_path(project_path, &context_path));

        match track {
            Track::QuickFlow => {
                // Quick Flow only needs a tech-spec stub
                let spec_path = planning_dir.join("tech-spec.md");
                fs::write(&spec_path, Self::template_tech_spec())?;
                files_created.push(Self::relative_path(project_path, &spec_path));
            }
            Track::BmadMethod => {
                let prd_path = planning_dir.join("PRD.md");
                fs::write(&prd_path, Self::template_prd(track))?;
                files_created.push(Self::relative_path(project_path, &prd_path));

                let arch_path = planning_dir.join("architecture.md");
                fs::write(&arch_path, Self::template_architecture(track))?;
                files_created.push(Self::relative_path(project_path, &arch_path));

                let epic_path = epics_dir.join("epic-1.md");
                fs::write(&epic_path, Self::template_epic())?;
                files_created.push(Self::relative_path(project_path, &epic_path));
            }
            Track::Enterprise => {
                let prd_path = planning_dir.join("PRD.md");
                fs::write(&prd_path, Self::template_prd(track))?;
                files_created.push(Self::relative_path(project_path, &prd_path));

                let arch_path = planning_dir.join("architecture.md");
                fs::write(&arch_path, Self::template_architecture(track))?;
                files_created.push(Self::relative_path(project_path, &arch_path));

                let epic_path = epics_dir.join("epic-1.md");
                fs::write(&epic_path, Self::template_epic())?;
                files_created.push(Self::relative_path(project_path, &epic_path));

                let security_path = planning_dir.join("security.md");
                fs::write(&security_path, Self::template_security())?;
                files_created.push(Self::relative_path(project_path, &security_path));

                let devops_path = planning_dir.join("devops.md");
                fs::write(&devops_path, Self::template_devops())?;
                files_created.push(Self::relative_path(project_path, &devops_path));
            }
        }

        let next_steps = match track {
            Track::QuickFlow => vec!["bmad-quick-dev"],
            Track::BmadMethod => vec!["bmad-create-prd", "bmad-create-architecture"],
            Track::Enterprise => vec![
                "bmad-create-prd",
                "bmad-create-architecture",
            ],
        };

        Ok(ScaffoldResult {
            files_created,
            track,
            next_steps,
        })
    }

    fn relative_path(base: &std::path::Path, full: &std::path::Path) -> String {
        full.strip_prefix(base)
            .unwrap_or(full)
            .to_string_lossy()
            .to_string()
    }

    fn template_project_context(track: Track) -> &'static str {
        match track {
            Track::QuickFlow => "\
# Project Context

## Track
Quick Flow

## Description
<!-- TODO: Briefly describe the project purpose and scope -->

## Goals
<!-- TODO: List the key goals for this project -->
- [ ] Goal 1
- [ ] Goal 2

## Constraints
<!-- TODO: List any known constraints (time, tech, budget) -->

## Notes
This project uses the BMad Method Quick Flow track, suitable for bug fixes,
simple features, or clear-scope work (1-15 stories).
",
            Track::BmadMethod => "\
# Project Context

## Track
BMad Method

## Description
<!-- TODO: Briefly describe the project purpose and scope -->

## Goals
<!-- TODO: List the key goals for this project -->
- [ ] Goal 1
- [ ] Goal 2

## Stakeholders
<!-- TODO: List key stakeholders and their roles -->

## Constraints
<!-- TODO: List any known constraints (time, tech, budget) -->

## Notes
This project uses the BMad Method track, suitable for products, platforms,
and complex features (10-50+ stories). It includes PRD, Architecture, and
UX design phases.
",
            Track::Enterprise => "\
# Project Context

## Track
Enterprise

## Description
<!-- TODO: Briefly describe the project purpose and scope -->

## Goals
<!-- TODO: List the key goals for this project -->
- [ ] Goal 1
- [ ] Goal 2

## Stakeholders
<!-- TODO: List key stakeholders and their roles -->

## Compliance Requirements
<!-- TODO: List regulatory or compliance requirements -->

## Constraints
<!-- TODO: List any known constraints (time, tech, budget) -->

## Notes
This project uses the BMad Method Enterprise track, suitable for compliance,
multi-tenant systems, and large-scale work (30+ stories). It includes PRD,
Architecture, Security, and DevOps phases.
",
        }
    }

    fn template_tech_spec() -> &'static str {
        "\
# Tech Spec

## Overview
<!-- TODO: Describe what this change does and why -->

## Scope
<!-- TODO: Define what is in scope and out of scope -->

### In Scope
- [ ] Item 1

### Out of Scope
- N/A

## Technical Approach
<!-- TODO: Describe the technical approach -->

## Testing Strategy
<!-- TODO: Describe how this will be tested -->
- [ ] Unit tests
- [ ] Integration tests

## Rollback Plan
<!-- TODO: Describe how to roll back if something goes wrong -->

## Checklist
- [ ] Tech spec reviewed
- [ ] Implementation complete
- [ ] Tests passing
- [ ] Code reviewed
"
    }

    fn template_prd(track: Track) -> &'static str {
        match track {
            Track::Enterprise => "\
# Product Requirements Document (PRD)

## 1. Executive Summary
<!-- TODO: High-level summary of the product/feature -->

## 2. Problem Statement
<!-- TODO: What problem does this solve? Who has this problem? -->

## 3. Goals and Success Metrics
<!-- TODO: Define measurable goals -->
| Goal | Metric | Target |
|------|--------|--------|
| <!-- TODO --> | <!-- TODO --> | <!-- TODO --> |

## 4. User Personas
<!-- TODO: Define target users -->

### Persona 1
- **Role:** <!-- TODO -->
- **Needs:** <!-- TODO -->
- **Pain Points:** <!-- TODO -->

## 5. Functional Requirements
<!-- TODO: List functional requirements with priority -->

### FR-1: <!-- TODO: Requirement title -->
- **Priority:** High / Medium / Low
- **Description:** <!-- TODO -->
- **Acceptance Criteria:**
  - [ ] <!-- TODO -->

## 6. Non-Functional Requirements
<!-- TODO: Performance, scalability, security, compliance -->

### Performance
- <!-- TODO: Response time, throughput targets -->

### Security
- <!-- TODO: Authentication, authorization, data protection -->

### Compliance
- <!-- TODO: Regulatory requirements (GDPR, SOC2, HIPAA, etc.) -->

### Scalability
- <!-- TODO: Expected load, growth projections -->

## 7. Dependencies
<!-- TODO: External systems, APIs, teams -->

## 8. Timeline
<!-- TODO: Key milestones -->

## 9. Risks and Mitigations
| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|-----------|
| <!-- TODO --> | <!-- TODO --> | <!-- TODO --> | <!-- TODO --> |

## 10. Open Questions
- [ ] <!-- TODO -->
",
            _ => "\
# Product Requirements Document (PRD)

## 1. Executive Summary
<!-- TODO: High-level summary of the product/feature -->

## 2. Problem Statement
<!-- TODO: What problem does this solve? Who has this problem? -->

## 3. Goals and Success Metrics
<!-- TODO: Define measurable goals -->
| Goal | Metric | Target |
|------|--------|--------|
| <!-- TODO --> | <!-- TODO --> | <!-- TODO --> |

## 4. User Personas
<!-- TODO: Define target users -->

### Persona 1
- **Role:** <!-- TODO -->
- **Needs:** <!-- TODO -->
- **Pain Points:** <!-- TODO -->

## 5. Functional Requirements
<!-- TODO: List functional requirements with priority -->

### FR-1: <!-- TODO: Requirement title -->
- **Priority:** High / Medium / Low
- **Description:** <!-- TODO -->
- **Acceptance Criteria:**
  - [ ] <!-- TODO -->

## 6. Non-Functional Requirements
<!-- TODO: Performance, scalability, security -->

## 7. Dependencies
<!-- TODO: External systems, APIs, teams -->

## 8. Timeline
<!-- TODO: Key milestones -->

## 9. Open Questions
- [ ] <!-- TODO -->
",
        }
    }

    fn template_architecture(track: Track) -> &'static str {
        match track {
            Track::Enterprise => "\
# Architecture Document

## 1. Overview
<!-- TODO: High-level architecture description -->

## 2. System Context
<!-- TODO: How this system fits into the broader ecosystem -->

## 3. Architecture Decisions (ADRs)

### ADR-1: <!-- TODO: Decision title -->
- **Status:** Proposed / Accepted / Deprecated
- **Context:** <!-- TODO: Why is this decision needed? -->
- **Decision:** <!-- TODO: What was decided? -->
- **Consequences:** <!-- TODO: What are the trade-offs? -->

## 4. Component Design
<!-- TODO: Key components and their responsibilities -->

## 5. Data Model
<!-- TODO: Core entities and relationships -->

## 6. API Design
<!-- TODO: Key API endpoints or interfaces -->

## 7. Security Architecture
<!-- TODO: Authentication, authorization, encryption, audit logging -->

### Authentication
- <!-- TODO -->

### Authorization
- <!-- TODO -->

### Data Protection
- <!-- TODO -->

### Audit Logging
- <!-- TODO -->

## 8. Infrastructure & DevOps
<!-- TODO: Deployment architecture, CI/CD, monitoring -->

### Deployment Architecture
- <!-- TODO -->

### CI/CD Pipeline
- <!-- TODO -->

### Monitoring & Alerting
- <!-- TODO -->

## 9. Scalability & Performance
<!-- TODO: Scaling strategy, caching, performance targets -->

## 10. Tech Stack
<!-- TODO: Languages, frameworks, databases, cloud services -->
| Layer | Technology | Rationale |
|-------|-----------|-----------|
| <!-- TODO --> | <!-- TODO --> | <!-- TODO --> |

## 11. Risks and Mitigations
| Risk | Impact | Mitigation |
|------|--------|-----------|
| <!-- TODO --> | <!-- TODO --> | <!-- TODO --> |
",
            _ => "\
# Architecture Document

## 1. Overview
<!-- TODO: High-level architecture description -->

## 2. Architecture Decisions (ADRs)

### ADR-1: <!-- TODO: Decision title -->
- **Status:** Proposed / Accepted / Deprecated
- **Context:** <!-- TODO: Why is this decision needed? -->
- **Decision:** <!-- TODO: What was decided? -->
- **Consequences:** <!-- TODO: What are the trade-offs? -->

## 3. Component Design
<!-- TODO: Key components and their responsibilities -->

## 4. Data Model
<!-- TODO: Core entities and relationships -->

## 5. API Design
<!-- TODO: Key API endpoints or interfaces -->

## 6. Tech Stack
<!-- TODO: Languages, frameworks, databases, cloud services -->
| Layer | Technology | Rationale |
|-------|-----------|-----------|
| <!-- TODO --> | <!-- TODO --> | <!-- TODO --> |

## 7. Deployment
<!-- TODO: How the system is deployed -->

## 8. Risks
| Risk | Impact | Mitigation |
|------|--------|-----------|
| <!-- TODO --> | <!-- TODO --> | <!-- TODO --> |
",
        }
    }

    fn template_epic() -> &'static str {
        "\
# Epic 1: <!-- TODO: Epic title -->

## Description
<!-- TODO: What does this epic deliver? -->

## Stories

### Story 1.1: <!-- TODO: Story title -->
- **Priority:** High / Medium / Low
- **Points:** <!-- TODO: Estimate -->
- **Description:** <!-- TODO -->
- **Acceptance Criteria:**
  - [ ] <!-- TODO -->
  - [ ] <!-- TODO -->

### Story 1.2: <!-- TODO: Story title -->
- **Priority:** High / Medium / Low
- **Points:** <!-- TODO: Estimate -->
- **Description:** <!-- TODO -->
- **Acceptance Criteria:**
  - [ ] <!-- TODO -->
  - [ ] <!-- TODO -->

## Dependencies
<!-- TODO: List dependencies on other epics or external systems -->

## Definition of Done
- [ ] All stories implemented and reviewed
- [ ] Tests passing
- [ ] Documentation updated
"
    }

    fn template_security() -> &'static str {
        "\
# Security Documentation

## 1. Threat Model
<!-- TODO: Identify key threats and attack vectors -->

### Assets
- <!-- TODO: What needs protection? -->

### Threat Actors
- <!-- TODO: Who might attack? -->

### Attack Vectors
| Vector | Likelihood | Impact | Mitigation |
|--------|-----------|--------|-----------|
| <!-- TODO --> | <!-- TODO --> | <!-- TODO --> | <!-- TODO --> |

## 2. Authentication & Authorization
<!-- TODO: Auth strategy -->

### Authentication
- <!-- TODO: Method (OAuth2, JWT, SAML, etc.) -->

### Authorization
- <!-- TODO: RBAC, ABAC, or other model -->

### Session Management
- <!-- TODO: Token lifecycle, refresh strategy -->

## 3. Data Protection
<!-- TODO: Encryption at rest and in transit -->

### Encryption at Rest
- <!-- TODO -->

### Encryption in Transit
- <!-- TODO -->

### PII Handling
- <!-- TODO: How is personally identifiable information handled? -->

## 4. Compliance
<!-- TODO: Regulatory requirements -->
- [ ] GDPR
- [ ] SOC2
- [ ] HIPAA
- [ ] Other: <!-- TODO -->

## 5. Audit & Monitoring
<!-- TODO: Security logging and monitoring strategy -->

## 6. Incident Response
<!-- TODO: Process for handling security incidents -->
"
    }

    fn template_devops() -> &'static str {
        "\
# DevOps Documentation

## 1. Infrastructure
<!-- TODO: Cloud provider, regions, architecture -->

### Environment Overview
| Environment | Purpose | URL |
|------------|---------|-----|
| Development | <!-- TODO --> | <!-- TODO --> |
| Staging | <!-- TODO --> | <!-- TODO --> |
| Production | <!-- TODO --> | <!-- TODO --> |

## 2. CI/CD Pipeline
<!-- TODO: Build, test, deploy pipeline -->

### Build
- <!-- TODO: Build tool and process -->

### Test
- <!-- TODO: Test stages (unit, integration, e2e) -->

### Deploy
- <!-- TODO: Deployment strategy (blue-green, canary, rolling) -->

## 3. Monitoring & Alerting
<!-- TODO: Observability stack -->

### Metrics
- <!-- TODO: Key metrics to track -->

### Logging
- <!-- TODO: Logging strategy and tools -->

### Alerting
- <!-- TODO: Alert rules and escalation -->

## 4. Disaster Recovery
<!-- TODO: Backup and recovery strategy -->

### Backup Strategy
- <!-- TODO: What is backed up, frequency, retention -->

### Recovery Objectives
- **RPO:** <!-- TODO: Recovery Point Objective -->
- **RTO:** <!-- TODO: Recovery Time Objective -->

## 5. Scaling Strategy
<!-- TODO: Auto-scaling rules and capacity planning -->

## 6. Runbooks
<!-- TODO: Link to operational runbooks for common tasks -->
- [ ] Deployment runbook
- [ ] Incident response runbook
- [ ] Scaling runbook
"
    }

    /// Search workflows, agents, and phases for a keyword (for bmad_help).
    pub fn search(&self, query: &str) -> Vec<String> {
        let lower = query.to_lowercase();
        let mut results = Vec::new();

        // Search agents
        for agent in self.agents.values() {
            if agent.name.to_lowercase().contains(&lower)
                || agent.skill_id.to_lowercase().contains(&lower)
                || agent.persona.to_lowercase().contains(&lower)
            {
                let workflows: Vec<&str> = agent.primary_workflows.clone();
                results.push(format!(
                    "**Agent: {} (persona: {}, skill: `{}`)**\n  Primary workflows: {}",
                    agent.name,
                    agent.persona,
                    agent.skill_id,
                    workflows.join(", "),
                ));
            }
        }

        // Search workflows
        for wf in self.workflows.values() {
            if wf.id.to_lowercase().contains(&lower)
                || wf.description.to_lowercase().contains(&lower)
                || wf.agent.to_lowercase().contains(&lower)
                || wf.produces.to_lowercase().contains(&lower)
            {
                let tracks: Vec<&str> = wf.tracks.iter().map(|t| t.name()).collect();
                results.push(format!(
                    "**Workflow: `{}`** — {}\n  Phase: {} | Agent: `{}` | Produces: {} | Tracks: {}",
                    wf.id,
                    wf.description,
                    wf.phase.name(),
                    wf.agent,
                    wf.produces,
                    tracks.join(", "),
                ));
            }
        }

        // Search phases
        for phase in Phase::all() {
            if phase.name().to_lowercase().contains(&lower) {
                results.push(format!(
                    "**Phase {}: {}** — {}",
                    phase.number(),
                    phase.name(),
                    phase.description(),
                ));
            }
        }

        // Search tracks
        for track in Track::all() {
            if track.name().to_lowercase().contains(&lower) {
                results.push(format!(
                    "**Track: {}** — {}",
                    track.name(),
                    track.description(),
                ));
            }
        }

        // Search core tools
        for tool in &self.core_tools {
            if tool.id.to_lowercase().contains(&lower)
                || tool.description.to_lowercase().contains(&lower)
            {
                results.push(format!(
                    "**Core Tool: `{}`** — {}",
                    tool.id, tool.description,
                ));
            }
        }

        results
    }

    // -----------------------------------------------------------------------
    // Static data builders
    // -----------------------------------------------------------------------

    fn build_agents() -> HashMap<&'static str, Agent> {
        let mut m = HashMap::new();

        m.insert(
            "bmad-analyst",
            Agent {
                name: "Analyst",
                persona: "Mary",
                skill_id: "bmad-analyst",
                primary_workflows: vec![
                    "bmad-brainstorming",
                    "bmad-market-research",
                    "bmad-domain-research",
                    "bmad-technical-research",
                    "bmad-create-product-brief",
                ],
            },
        );
        m.insert(
            "bmad-pm",
            Agent {
                name: "Product Manager",
                persona: "John",
                skill_id: "bmad-pm",
                primary_workflows: vec![
                    "bmad-create-prd",
                    "bmad-create-epics-and-stories",
                    "bmad-check-implementation-readiness",
                    "bmad-correct-course",
                ],
            },
        );
        m.insert(
            "bmad-architect",
            Agent {
                name: "Architect",
                persona: "Winston",
                skill_id: "bmad-architect",
                primary_workflows: vec![
                    "bmad-create-architecture",
                    "bmad-check-implementation-readiness",
                ],
            },
        );
        m.insert(
            "bmad-sm",
            Agent {
                name: "Scrum Master",
                persona: "Bob",
                skill_id: "bmad-sm",
                primary_workflows: vec![
                    "bmad-sprint-planning",
                    "bmad-create-story",
                    "bmad-retrospective",
                    "bmad-correct-course",
                ],
            },
        );
        m.insert(
            "bmad-dev",
            Agent {
                name: "Developer",
                persona: "Amelia",
                skill_id: "bmad-dev",
                primary_workflows: vec!["bmad-dev-story", "bmad-code-review"],
            },
        );
        m.insert(
            "bmad-qa",
            Agent {
                name: "QA Engineer",
                persona: "Quinn",
                skill_id: "bmad-qa",
                primary_workflows: vec!["bmad-automate"],
            },
        );
        m.insert(
            "bmad-master",
            Agent {
                name: "Quick Flow Solo Dev",
                persona: "Barry",
                skill_id: "bmad-master",
                primary_workflows: vec!["bmad-quick-dev", "bmad-code-review"],
            },
        );
        m.insert(
            "bmad-ux-designer",
            Agent {
                name: "UX Designer",
                persona: "Sally",
                skill_id: "bmad-ux-designer",
                primary_workflows: vec!["bmad-create-ux-design"],
            },
        );
        m.insert(
            "bmad-tech-writer",
            Agent {
                name: "Technical Writer",
                persona: "Paige",
                skill_id: "bmad-tech-writer",
                primary_workflows: vec![
                    "bmad-document-project",
                    "bmad-write-document",
                    "bmad-update-standards",
                    "bmad-mermaid-generate",
                    "bmad-validate-doc",
                    "bmad-explain-concept",
                ],
            },
        );

        m
    }

    fn build_workflows() -> HashMap<&'static str, Workflow> {
        let mut m = HashMap::new();

        let all_tracks = vec![Track::QuickFlow, Track::BmadMethod, Track::Enterprise];
        let full_tracks = vec![Track::BmadMethod, Track::Enterprise];

        // Phase 1: Analysis
        m.insert(
            "bmad-brainstorming",
            Workflow {
                id: "bmad-brainstorming",
                description: "Brainstorm Project Ideas with guided facilitation",
                phase: Phase::Analysis,
                agent: "bmad-analyst",
                produces: "brainstorming-report.md",
                prerequisites: vec![],
                next_steps: vec![
                    "bmad-market-research",
                    "bmad-domain-research",
                    "bmad-technical-research",
                    "bmad-create-product-brief",
                ],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-market-research",
            Workflow {
                id: "bmad-market-research",
                description: "Validate market assumptions",
                phase: Phase::Analysis,
                agent: "bmad-analyst",
                produces: "research findings",
                prerequisites: vec![],
                next_steps: vec!["bmad-create-product-brief"],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-domain-research",
            Workflow {
                id: "bmad-domain-research",
                description: "Validate domain assumptions",
                phase: Phase::Analysis,
                agent: "bmad-analyst",
                produces: "research findings",
                prerequisites: vec![],
                next_steps: vec!["bmad-create-product-brief"],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-technical-research",
            Workflow {
                id: "bmad-technical-research",
                description: "Validate technical assumptions",
                phase: Phase::Analysis,
                agent: "bmad-analyst",
                produces: "research findings",
                prerequisites: vec![],
                next_steps: vec!["bmad-create-product-brief"],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-create-product-brief",
            Workflow {
                id: "bmad-create-product-brief",
                description: "Capture strategic vision",
                phase: Phase::Analysis,
                agent: "bmad-analyst",
                produces: "product-brief.md",
                prerequisites: vec![],
                next_steps: vec!["bmad-create-prd"],
                tracks: full_tracks.clone(),
            },
        );

        // Phase 2: Planning
        m.insert(
            "bmad-create-prd",
            Workflow {
                id: "bmad-create-prd",
                description: "Define requirements (functional and non-functional)",
                phase: Phase::Planning,
                agent: "bmad-pm",
                produces: "PRD.md",
                prerequisites: vec![],
                next_steps: vec!["bmad-create-ux-design", "bmad-create-architecture"],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-create-ux-design",
            Workflow {
                id: "bmad-create-ux-design",
                description: "Design user experience (when UX matters)",
                phase: Phase::Planning,
                agent: "bmad-ux-designer",
                produces: "ux-spec.md",
                prerequisites: vec!["bmad-create-prd"],
                next_steps: vec!["bmad-create-architecture"],
                tracks: full_tracks.clone(),
            },
        );

        // Phase 3: Solutioning
        m.insert(
            "bmad-create-architecture",
            Workflow {
                id: "bmad-create-architecture",
                description: "Make technical decisions explicit",
                phase: Phase::Solutioning,
                agent: "bmad-architect",
                produces: "architecture.md with ADRs",
                prerequisites: vec!["bmad-create-prd"],
                next_steps: vec!["bmad-create-epics-and-stories"],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-create-epics-and-stories",
            Workflow {
                id: "bmad-create-epics-and-stories",
                description: "Break requirements into implementable work",
                phase: Phase::Solutioning,
                agent: "bmad-pm",
                produces: "Epic files with stories",
                prerequisites: vec!["bmad-create-prd", "bmad-create-architecture"],
                next_steps: vec!["bmad-check-implementation-readiness"],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-check-implementation-readiness",
            Workflow {
                id: "bmad-check-implementation-readiness",
                description: "Gate check before implementation",
                phase: Phase::Solutioning,
                agent: "bmad-architect",
                produces: "PASS/CONCERNS/FAIL decision",
                prerequisites: vec!["bmad-create-epics-and-stories"],
                next_steps: vec!["bmad-sprint-planning"],
                tracks: full_tracks.clone(),
            },
        );

        // Phase 4: Implementation
        m.insert(
            "bmad-sprint-planning",
            Workflow {
                id: "bmad-sprint-planning",
                description:
                    "Initialize tracking (once per project to sequence the dev cycle)",
                phase: Phase::Implementation,
                agent: "bmad-sm",
                produces: "sprint-status.yaml",
                prerequisites: vec!["bmad-check-implementation-readiness"],
                next_steps: vec!["bmad-create-story"],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-create-story",
            Workflow {
                id: "bmad-create-story",
                description: "Prepare next story for implementation",
                phase: Phase::Implementation,
                agent: "bmad-sm",
                produces: "story-[slug].md",
                prerequisites: vec!["bmad-sprint-planning"],
                next_steps: vec!["bmad-dev-story"],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-dev-story",
            Workflow {
                id: "bmad-dev-story",
                description: "Implement the story",
                phase: Phase::Implementation,
                agent: "bmad-dev",
                produces: "Working code + tests",
                prerequisites: vec!["bmad-create-story"],
                next_steps: vec!["bmad-code-review"],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-code-review",
            Workflow {
                id: "bmad-code-review",
                description: "Validate implementation quality",
                phase: Phase::Implementation,
                agent: "bmad-dev",
                produces: "Approved or changes requested",
                prerequisites: vec!["bmad-dev-story"],
                next_steps: vec!["bmad-create-story", "bmad-retrospective"],
                tracks: all_tracks.clone(),
            },
        );
        m.insert(
            "bmad-correct-course",
            Workflow {
                id: "bmad-correct-course",
                description: "Handle significant mid-sprint changes",
                phase: Phase::Implementation,
                agent: "bmad-sm",
                produces: "Updated plan or re-routing",
                prerequisites: vec![],
                next_steps: vec![],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-sprint-status",
            Workflow {
                id: "bmad-sprint-status",
                description: "Track sprint progress and story status",
                phase: Phase::Implementation,
                agent: "bmad-sm",
                produces: "Sprint status update",
                prerequisites: vec!["bmad-sprint-planning"],
                next_steps: vec![],
                tracks: full_tracks.clone(),
            },
        );
        m.insert(
            "bmad-retrospective",
            Workflow {
                id: "bmad-retrospective",
                description: "Review after epic completion",
                phase: Phase::Implementation,
                agent: "bmad-sm",
                produces: "Lessons learned",
                prerequisites: vec![],
                next_steps: vec!["bmad-sprint-planning"],
                tracks: full_tracks,
            },
        );

        // Quick Flow (parallel track)
        m.insert(
            "bmad-quick-dev",
            Workflow {
                id: "bmad-quick-dev",
                description:
                    "Unified quick flow — clarify intent, plan, implement, review, and present",
                phase: Phase::Implementation,
                agent: "bmad-master",
                produces: "spec-*.md + code",
                prerequisites: vec![],
                next_steps: vec!["bmad-code-review"],
                tracks: vec![Track::QuickFlow],
            },
        );

        m
    }

    fn build_core_tools() -> Vec<CoreTool> {
        vec![
            CoreTool {
                id: "bmad-help",
                description: "Context-aware guidance on next steps",
            },
            CoreTool {
                id: "bmad-party-mode",
                description: "Multi-agent group discussions",
            },
            CoreTool {
                id: "bmad-brainstorming",
                description: "Facilitated creative sessions",
            },
            CoreTool {
                id: "bmad-distillator",
                description: "Lossless document compression",
            },
            CoreTool {
                id: "bmad-advanced-elicitation",
                description: "Iterative content refinement methods",
            },
            CoreTool {
                id: "bmad-review-adversarial-general",
                description: "Cynical review finding gaps and issues",
            },
            CoreTool {
                id: "bmad-review-edge-case-hunter",
                description: "Exhaustive path and boundary analysis",
            },
            CoreTool {
                id: "bmad-editorial-review-prose",
                description: "Copy-editing for clarity",
            },
            CoreTool {
                id: "bmad-editorial-review-structure",
                description: "Structural reorganization recommendations",
            },
            CoreTool {
                id: "bmad-shard-doc",
                description: "Split large docs into sections",
            },
            CoreTool {
                id: "bmad-index-docs",
                description: "Generate/update document indices",
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> BmadIndex {
        BmadIndex::build()
    }

    // ------------------------------------------------------------------
    // Workflow lookup
    // ------------------------------------------------------------------

    #[test]
    fn get_workflow_returns_correct_metadata() {
        let idx = index();
        let wf = idx.get_workflow("bmad-create-prd").unwrap();

        assert_eq!(wf.id, "bmad-create-prd");
        assert_eq!(wf.phase, Phase::Planning);
        assert_eq!(wf.agent, "bmad-pm");
        assert_eq!(wf.produces, "PRD.md");
        assert!(wf.tracks.contains(&Track::BmadMethod));
        assert!(wf.tracks.contains(&Track::Enterprise));
        assert!(!wf.tracks.contains(&Track::QuickFlow));
    }

    #[test]
    fn get_workflow_unknown_returns_none() {
        let idx = index();
        assert!(idx.get_workflow("nonexistent").is_none());
    }

    #[test]
    fn all_workflow_ids_is_nonempty() {
        let idx = index();
        let ids = idx.all_workflow_ids();
        assert!(ids.len() >= 18, "expected at least 18 workflows");
    }

    // ------------------------------------------------------------------
    // Phase next-steps
    // ------------------------------------------------------------------

    #[test]
    fn get_next_steps_analysis() {
        let idx = index();
        let steps = idx.get_next_steps(Phase::Analysis);
        assert_eq!(steps, vec!["bmad-create-prd", "bmad-quick-dev"]);
    }

    #[test]
    fn get_next_steps_planning() {
        let idx = index();
        let steps = idx.get_next_steps(Phase::Planning);
        assert!(steps.contains(&"bmad-create-architecture"));
        assert!(steps.contains(&"bmad-quick-dev"));
    }

    #[test]
    fn get_next_steps_solutioning() {
        let idx = index();
        let steps = idx.get_next_steps(Phase::Solutioning);
        assert_eq!(steps, vec!["bmad-sprint-planning"]);
    }

    #[test]
    fn get_next_steps_implementation() {
        let idx = index();
        let steps = idx.get_next_steps(Phase::Implementation);
        assert_eq!(steps, vec!["bmad-retrospective"]);
    }

    // ------------------------------------------------------------------
    // Workflow next-steps
    // ------------------------------------------------------------------

    #[test]
    fn workflow_next_steps_create_prd() {
        let idx = index();
        let steps = idx.get_workflow_next_steps("bmad-create-prd");
        assert!(steps.contains(&"bmad-create-architecture"));
        assert!(steps.contains(&"bmad-create-ux-design"));
    }

    #[test]
    fn workflow_next_steps_create_architecture() {
        let idx = index();
        let steps = idx.get_workflow_next_steps("bmad-create-architecture");
        assert_eq!(steps, vec!["bmad-create-epics-and-stories"]);
    }

    #[test]
    fn workflow_next_steps_quick_dev() {
        let idx = index();
        let steps = idx.get_workflow_next_steps("bmad-quick-dev");
        assert_eq!(steps, vec!["bmad-code-review"]);
    }

    // ------------------------------------------------------------------
    // Phase workflows
    // ------------------------------------------------------------------

    #[test]
    fn phase_analysis_workflows() {
        let idx = index();
        let wfs = idx.get_phase_workflows(Phase::Analysis);
        assert!(wfs.contains(&"bmad-brainstorming"));
        assert!(wfs.contains(&"bmad-create-product-brief"));
        assert!(wfs.contains(&"bmad-market-research"));
    }

    #[test]
    fn phase_planning_workflows() {
        let idx = index();
        let wfs = idx.get_phase_workflows(Phase::Planning);
        assert!(wfs.contains(&"bmad-create-prd"));
        assert!(wfs.contains(&"bmad-create-ux-design"));
    }

    #[test]
    fn phase_solutioning_workflows() {
        let idx = index();
        let wfs = idx.get_phase_workflows(Phase::Solutioning);
        assert!(wfs.contains(&"bmad-create-architecture"));
        assert!(wfs.contains(&"bmad-create-epics-and-stories"));
        assert!(wfs.contains(&"bmad-check-implementation-readiness"));
    }

    #[test]
    fn phase_implementation_workflows() {
        let idx = index();
        let wfs = idx.get_phase_workflows(Phase::Implementation);
        assert!(wfs.contains(&"bmad-sprint-planning"));
        assert!(wfs.contains(&"bmad-create-story"));
        assert!(wfs.contains(&"bmad-dev-story"));
        assert!(wfs.contains(&"bmad-code-review"));
        assert!(wfs.contains(&"bmad-quick-dev"));
    }

    // ------------------------------------------------------------------
    // Track workflows
    // ------------------------------------------------------------------

    #[test]
    fn quick_flow_track_has_quick_dev() {
        let idx = index();
        let wfs = idx.get_track_workflows(Track::QuickFlow);
        let ids: Vec<&str> = wfs.iter().map(|w| w.id).collect();
        assert!(ids.contains(&"bmad-quick-dev"));
        // Quick flow should also include code-review (shared)
        assert!(ids.contains(&"bmad-code-review"));
    }

    #[test]
    fn bmad_method_track_has_full_workflow_chain() {
        let idx = index();
        let wfs = idx.get_track_workflows(Track::BmadMethod);
        let ids: Vec<&str> = wfs.iter().map(|w| w.id).collect();
        assert!(ids.contains(&"bmad-create-prd"));
        assert!(ids.contains(&"bmad-create-architecture"));
        assert!(ids.contains(&"bmad-create-epics-and-stories"));
        assert!(ids.contains(&"bmad-sprint-planning"));
        assert!(ids.contains(&"bmad-dev-story"));
    }

    #[test]
    fn enterprise_track_has_full_workflow_chain() {
        let idx = index();
        let wfs = idx.get_track_workflows(Track::Enterprise);
        let ids: Vec<&str> = wfs.iter().map(|w| w.id).collect();
        assert!(ids.contains(&"bmad-create-prd"));
        assert!(ids.contains(&"bmad-create-architecture"));
        assert!(ids.contains(&"bmad-check-implementation-readiness"));
    }

    // ------------------------------------------------------------------
    // Agents
    // ------------------------------------------------------------------

    #[test]
    fn get_agent_dev() {
        let idx = index();
        let agent = idx.get_agent("bmad-dev").unwrap();
        assert_eq!(agent.name, "Developer");
        assert_eq!(agent.persona, "Amelia");
        assert!(agent.primary_workflows.contains(&"bmad-dev-story"));
    }

    #[test]
    fn all_agents_count() {
        let idx = index();
        let agents = idx.all_agents();
        assert_eq!(agents.len(), 9, "expected 9 default agents");
    }

    #[test]
    fn all_agents_sorted_by_skill_id() {
        let idx = index();
        let agents = idx.all_agents();
        let ids: Vec<&str> = agents.iter().map(|a| a.skill_id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "all_agents should return agents sorted by skill_id");
    }

    #[test]
    fn get_agent_pm() {
        let idx = index();
        let agent = idx.get_agent("bmad-pm").unwrap();
        assert_eq!(agent.name, "Product Manager");
        assert_eq!(agent.persona, "John");
        assert!(agent.primary_workflows.contains(&"bmad-create-prd"));
        assert!(agent.primary_workflows.contains(&"bmad-create-epics-and-stories"));
    }

    #[test]
    fn get_agent_architect() {
        let idx = index();
        let agent = idx.get_agent("bmad-architect").unwrap();
        assert_eq!(agent.name, "Architect");
        assert_eq!(agent.persona, "Winston");
        assert!(agent.primary_workflows.contains(&"bmad-create-architecture"));
    }

    #[test]
    fn get_agent_unknown_returns_none() {
        let idx = index();
        assert!(idx.get_agent("nonexistent").is_none());
    }

    #[test]
    fn agents_by_phase_analysis() {
        let idx = index();
        let agents = idx.get_agents_by_phase(Phase::Analysis);
        let ids: Vec<&str> = agents.iter().map(|a| a.skill_id).collect();
        assert!(ids.contains(&"bmad-analyst"), "Analyst should be in Analysis phase");
        assert!(!ids.contains(&"bmad-dev"), "Developer should not be in Analysis phase");
    }

    #[test]
    fn agents_by_phase_implementation() {
        let idx = index();
        let agents = idx.get_agents_by_phase(Phase::Implementation);
        let ids: Vec<&str> = agents.iter().map(|a| a.skill_id).collect();
        assert!(ids.contains(&"bmad-dev"), "Developer should be in Implementation phase");
        assert!(ids.contains(&"bmad-sm"), "Scrum Master should be in Implementation phase");
        assert!(ids.contains(&"bmad-master"), "Quick Flow Solo Dev should be in Implementation phase");
    }

    #[test]
    fn agents_by_phase_solutioning() {
        let idx = index();
        let agents = idx.get_agents_by_phase(Phase::Solutioning);
        let ids: Vec<&str> = agents.iter().map(|a| a.skill_id).collect();
        assert!(ids.contains(&"bmad-architect"), "Architect should be in Solutioning phase");
        assert!(ids.contains(&"bmad-pm"), "PM should be in Solutioning phase");
    }

    #[test]
    fn agents_by_phase_sorted() {
        let idx = index();
        let agents = idx.get_agents_by_phase(Phase::Implementation);
        let ids: Vec<&str> = agents.iter().map(|a| a.skill_id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "agents_by_phase should return sorted results");
    }

    // ------------------------------------------------------------------
    // Core tools
    // ------------------------------------------------------------------

    #[test]
    fn core_tools_includes_help() {
        let idx = index();
        let ids: Vec<&str> = idx.core_tools().iter().map(|t| t.id).collect();
        assert!(ids.contains(&"bmad-help"));
    }

    #[test]
    fn core_tools_count() {
        let idx = index();
        assert_eq!(idx.core_tools().len(), 11, "expected 11 core tools");
    }

    // ------------------------------------------------------------------
    // Phases
    // ------------------------------------------------------------------

    #[test]
    fn phase_metadata() {
        assert_eq!(Phase::Analysis.number(), 1);
        assert_eq!(Phase::Planning.number(), 2);
        assert_eq!(Phase::Solutioning.number(), 3);
        assert_eq!(Phase::Implementation.number(), 4);

        assert!(Phase::Analysis.is_optional());
        assert!(!Phase::Planning.is_optional());
    }

    #[test]
    fn all_phases() {
        assert_eq!(Phase::all().len(), 4);
    }

    // ------------------------------------------------------------------
    // Tracks
    // ------------------------------------------------------------------

    #[test]
    fn track_phases() {
        assert_eq!(Track::QuickFlow.phases(), vec![Phase::Implementation]);
        assert_eq!(Track::BmadMethod.phases().len(), 4);
        assert_eq!(Track::Enterprise.phases().len(), 4);
    }

    #[test]
    fn all_tracks() {
        assert_eq!(Track::all().len(), 3);
    }

    // ------------------------------------------------------------------
    // Raw docs
    // ------------------------------------------------------------------

    #[test]
    fn raw_docs_is_nonempty() {
        let idx = BmadIndex::build();
        let docs = idx.raw_docs();
        assert!(docs.len() > 1000, "embedded docs should be > 1000 chars");
        assert!(docs.contains("BMad Method"));
    }

    // ------------------------------------------------------------------
    // State inference / next-step recommendation
    // ------------------------------------------------------------------

    #[test]
    fn infer_completed_from_prd_and_architecture() {
        let completed = BmadIndex::infer_completed_workflows("has PRD, has architecture");
        assert!(completed.contains(&"bmad-create-prd"));
        assert!(completed.contains(&"bmad-create-architecture"));
        assert!(!completed.contains(&"bmad-create-ux-design"));
    }

    #[test]
    fn infer_completed_from_empty_state() {
        let completed = BmadIndex::infer_completed_workflows("nothing yet");
        assert!(completed.is_empty());
    }

    #[test]
    fn infer_completed_is_case_insensitive() {
        let completed = BmadIndex::infer_completed_workflows("has prd and ARCHITECTURE and UX spec");
        assert!(completed.contains(&"bmad-create-prd"));
        assert!(completed.contains(&"bmad-create-architecture"));
        assert!(completed.contains(&"bmad-create-ux-design"));
    }

    #[test]
    fn infer_completed_no_duplicates() {
        // "prd" appears once but should only yield one entry
        let completed = BmadIndex::infer_completed_workflows("has PRD");
        let count = completed.iter().filter(|&&id| id == "bmad-create-prd").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn infer_phase_empty_is_analysis() {
        let phase = BmadIndex::infer_current_phase(&[]);
        assert_eq!(phase, Phase::Analysis);
    }

    #[test]
    fn infer_phase_after_prd() {
        let phase = BmadIndex::infer_current_phase(&["bmad-create-prd"]);
        assert_eq!(phase, Phase::Planning);
    }

    #[test]
    fn infer_phase_after_architecture() {
        let phase = BmadIndex::infer_current_phase(&["bmad-create-prd", "bmad-create-architecture"]);
        assert_eq!(phase, Phase::Solutioning);
    }

    #[test]
    fn infer_phase_after_sprint_planning() {
        let phase = BmadIndex::infer_current_phase(&[
            "bmad-create-prd",
            "bmad-create-architecture",
            "bmad-sprint-planning",
        ]);
        assert_eq!(phase, Phase::Implementation);
    }

    #[test]
    fn recommend_next_from_scratch() {
        let idx = index();
        let recs = idx.recommend_next(&[], None);
        // From Analysis phase, should suggest PRD or quick-dev
        let ids: Vec<&str> = recs.iter().map(|w| w.id).collect();
        assert!(
            ids.contains(&"bmad-create-prd") || ids.contains(&"bmad-brainstorming"),
            "expected analysis/planning workflows but got: {ids:?}"
        );
    }

    #[test]
    fn recommend_next_after_prd() {
        let idx = index();
        let recs = idx.recommend_next(
            &["bmad-create-prd"],
            Some("bmad-create-prd"),
        );
        let ids: Vec<&str> = recs.iter().map(|w| w.id).collect();
        assert!(ids.contains(&"bmad-create-architecture") || ids.contains(&"bmad-create-ux-design"));
    }

    // ------------------------------------------------------------------
    // Search
    // ------------------------------------------------------------------

    #[test]
    fn search_finds_sm_agent() {
        let idx = index();
        let results = idx.search("scrum master");
        assert!(!results.is_empty());
        let joined = results.join("\n");
        assert!(joined.contains("Scrum Master") || joined.contains("bmad-sm"));
    }

    #[test]
    fn search_finds_phase() {
        let idx = index();
        let results = idx.search("analysis");
        assert!(!results.is_empty());
    }

    // ------------------------------------------------------------------
    // Readiness check
    // ------------------------------------------------------------------

    // Quick Flow track

    #[test]
    fn readiness_quick_flow_missing_tech_spec() {
        let result = BmadIndex::check_readiness("nothing yet", Track::QuickFlow);
        assert!(!result.ready);
        assert_eq!(result.missing_artifacts, vec!["tech-spec"]);
        assert!(result.next_action.contains("bmad-quick-dev"));
    }

    #[test]
    fn readiness_quick_flow_has_tech_spec() {
        let result = BmadIndex::check_readiness("tech-spec done", Track::QuickFlow);
        assert!(result.ready);
        assert!(result.missing_artifacts.is_empty());
        assert!(result.next_action.contains("bmad-quick-dev"));
    }

    #[test]
    fn readiness_quick_flow_has_spec_dot_variant() {
        let result = BmadIndex::check_readiness("spec.md written", Track::QuickFlow);
        assert!(result.ready);
    }

    // BMad Method track

    #[test]
    fn readiness_bmad_all_present() {
        let result = BmadIndex::check_readiness(
            "PRD.md done, architecture.md done, epics created",
            Track::BmadMethod,
        );
        assert!(result.ready);
        assert!(result.missing_artifacts.is_empty());
        assert!(result.next_action.contains("bmad-sprint-planning"));
    }

    #[test]
    fn readiness_bmad_missing_prd() {
        let result = BmadIndex::check_readiness(
            "architecture.md done, epics created",
            Track::BmadMethod,
        );
        assert!(!result.ready);
        assert!(result.missing_artifacts.contains(&"PRD".to_string()));
        assert!(!result.missing_artifacts.contains(&"architecture".to_string()));
        assert!(!result.missing_artifacts.contains(&"epics/stories".to_string()));
        assert!(result.next_action.contains("bmad-create-prd"));
    }

    #[test]
    fn readiness_bmad_missing_architecture() {
        let result = BmadIndex::check_readiness("PRD done, epics created", Track::BmadMethod);
        assert!(!result.ready);
        assert!(result.missing_artifacts.contains(&"architecture".to_string()));
        assert!(!result.missing_artifacts.contains(&"PRD".to_string()));
    }

    #[test]
    fn readiness_bmad_missing_epics() {
        let result = BmadIndex::check_readiness(
            "PRD done, architecture done",
            Track::BmadMethod,
        );
        assert!(!result.ready);
        assert!(result.missing_artifacts.contains(&"epics/stories".to_string()));
    }

    #[test]
    fn readiness_bmad_missing_all() {
        let result = BmadIndex::check_readiness("nothing yet", Track::BmadMethod);
        assert!(!result.ready);
        assert_eq!(result.missing_artifacts.len(), 3);
        assert!(result.missing_artifacts.contains(&"PRD".to_string()));
        assert!(result.missing_artifacts.contains(&"architecture".to_string()));
        assert!(result.missing_artifacts.contains(&"epics/stories".to_string()));
    }

    #[test]
    fn readiness_bmad_warns_about_optional() {
        let result = BmadIndex::check_readiness(
            "PRD done, architecture done, epics created",
            Track::BmadMethod,
        );
        assert!(result.ready);
        assert!(!result.warnings.is_empty());
        let warnings_text = result.warnings.join(" ");
        assert!(warnings_text.contains("UX spec"));
        assert!(warnings_text.contains("implementation readiness check"));
    }

    #[test]
    fn readiness_bmad_no_warnings_when_optional_present() {
        let result = BmadIndex::check_readiness(
            "PRD done, architecture done, epics created, UX spec done, readiness check passed",
            Track::BmadMethod,
        );
        assert!(result.ready);
        assert!(result.warnings.is_empty());
    }

    // Enterprise track

    #[test]
    fn readiness_enterprise_all_present() {
        let result = BmadIndex::check_readiness(
            "PRD done, architecture done, security docs done, DevOps docs done, epics created",
            Track::Enterprise,
        );
        assert!(result.ready);
        assert!(result.missing_artifacts.is_empty());
        assert!(result.next_action.contains("bmad-sprint-planning"));
    }

    #[test]
    fn readiness_enterprise_missing_security() {
        let result = BmadIndex::check_readiness(
            "PRD done, architecture done, DevOps docs done, epics created",
            Track::Enterprise,
        );
        assert!(!result.ready);
        assert!(result.missing_artifacts.contains(&"security docs".to_string()));
    }

    #[test]
    fn readiness_enterprise_missing_devops() {
        let result = BmadIndex::check_readiness(
            "PRD done, architecture done, security docs done, epics created",
            Track::Enterprise,
        );
        assert!(!result.ready);
        assert!(result.missing_artifacts.contains(&"DevOps docs".to_string()));
    }

    #[test]
    fn readiness_enterprise_missing_multiple() {
        let result = BmadIndex::check_readiness("PRD done", Track::Enterprise);
        assert!(!result.ready);
        assert!(result.missing_artifacts.len() >= 3);
        assert!(result.missing_artifacts.contains(&"architecture".to_string()));
        assert!(result.missing_artifacts.contains(&"security docs".to_string()));
        assert!(result.missing_artifacts.contains(&"DevOps docs".to_string()));
        assert!(result.missing_artifacts.contains(&"epics/stories".to_string()));
    }

    #[test]
    fn readiness_enterprise_cicd_counts_as_devops() {
        let result = BmadIndex::check_readiness(
            "PRD done, architecture done, security docs done, CI/CD configured, epics created",
            Track::Enterprise,
        );
        assert!(result.ready);
    }

    // Default track

    #[test]
    fn readiness_case_insensitive() {
        let result = BmadIndex::check_readiness(
            "prd done, ARCHITECTURE done, Epics created",
            Track::BmadMethod,
        );
        assert!(result.ready);
    }

    // -----------------------------------------------------------------------
    // Sprint guide
    // -----------------------------------------------------------------------

    #[test]
    fn sprint_guide_story_created_not_implemented() {
        let r = BmadIndex::sprint_guide("story file created but not yet implemented");
        assert_eq!(r.current_step, "Story implementation");
        assert_eq!(r.agent_to_invoke, "bmad-agent-dev");
        assert_eq!(r.workflow_to_run, "bmad-dev-story");
    }

    #[test]
    fn sprint_guide_story_implemented_not_reviewed() {
        let r = BmadIndex::sprint_guide("story implemented, not reviewed");
        assert_eq!(r.current_step, "Code review");
        assert_eq!(r.agent_to_invoke, "bmad-agent-dev");
        assert_eq!(r.workflow_to_run, "bmad-code-review");
    }

    #[test]
    fn sprint_guide_all_stories_in_epic_done() {
        let r = BmadIndex::sprint_guide("all stories in epic done");
        assert_eq!(r.current_step, "Epic retrospective");
        assert_eq!(r.agent_to_invoke, "bmad-agent-sm");
        assert_eq!(r.workflow_to_run, "bmad-retrospective");
    }

    #[test]
    fn sprint_guide_no_sprint_plan() {
        let r = BmadIndex::sprint_guide("no sprint plan exists yet");
        assert_eq!(r.current_step, "Sprint initialization");
        assert_eq!(r.agent_to_invoke, "bmad-agent-sm");
        assert_eq!(r.workflow_to_run, "bmad-sprint-planning");
    }

    #[test]
    fn sprint_guide_reviewed_more_stories_remain() {
        let r = BmadIndex::sprint_guide("story reviewed, more stories remain in the epic");
        assert_eq!(r.current_step, "Create next story");
        assert_eq!(r.agent_to_invoke, "bmad-agent-sm");
        assert_eq!(r.workflow_to_run, "bmad-create-story");
    }

    #[test]
    fn sprint_guide_no_story_exists() {
        let r = BmadIndex::sprint_guide("no story file, need to create story for epic 2");
        assert_eq!(r.current_step, "Create story");
        assert_eq!(r.agent_to_invoke, "bmad-agent-sm");
        assert_eq!(r.workflow_to_run, "bmad-create-story");
    }

    #[test]
    fn sprint_guide_retrospective_done() {
        let r = BmadIndex::sprint_guide("retrospective done, moving to next epic");
        assert_eq!(r.current_step, "Start next epic");
        assert_eq!(r.agent_to_invoke, "bmad-agent-sm");
        assert_eq!(r.workflow_to_run, "bmad-create-story");
    }

    #[test]
    fn sprint_guide_has_sprint_plan_fallback() {
        let r = BmadIndex::sprint_guide("sprint plan ready, epic 1 queued");
        assert_eq!(r.current_step, "Create story");
        assert_eq!(r.agent_to_invoke, "bmad-agent-sm");
        assert_eq!(r.workflow_to_run, "bmad-create-story");
    }

    #[test]
    fn sprint_guide_unknown_state_fallback() {
        let r = BmadIndex::sprint_guide("some random text with no keywords");
        assert_eq!(r.current_step, "Sprint initialization");
        assert_eq!(r.agent_to_invoke, "bmad-agent-sm");
        assert_eq!(r.workflow_to_run, "bmad-sprint-planning");
    }

    #[test]
    fn sprint_guide_case_insensitive() {
        let r = BmadIndex::sprint_guide("Story File Created but NOT IMPLEMENTED");
        assert_eq!(r.workflow_to_run, "bmad-dev-story");
    }

    #[test]
    fn sprint_guide_epic_complete_synonym() {
        let r = BmadIndex::sprint_guide("epic 1 complete, all stories reviewed");
        assert_eq!(r.workflow_to_run, "bmad-retrospective");
    }

    #[test]
    fn sprint_guide_coded_not_reviewed() {
        let r = BmadIndex::sprint_guide("story 3 coded and ready for review");
        assert_eq!(r.current_step, "Code review");
        assert_eq!(r.workflow_to_run, "bmad-code-review");
    }

    #[test]
    fn sprint_guide_complex_state_description() {
        // "epic 1 complete" doesn't match "epic complete" (number in between).
        // "story file created but not yet implemented" properly detects story implementation.
        let r = BmadIndex::sprint_guide(
            "epic 1 complete, working on epic 2 story 3, story file created but not yet implemented",
        );
        assert_eq!(r.workflow_to_run, "bmad-dev-story");
    }

    #[test]
    fn sprint_guide_story_created_for_new_epic() {
        // Without "epic complete", story creation state is detected properly.
        let r = BmadIndex::sprint_guide(
            "working on epic 2 story 3, story file created but not yet implemented",
        );
        assert_eq!(r.workflow_to_run, "bmad-dev-story");
    }

    #[test]
    fn sprint_guide_not_started() {
        let r = BmadIndex::sprint_guide("not started yet, brand new project");
        assert_eq!(r.workflow_to_run, "bmad-sprint-planning");
    }

    // ------------------------------------------------------------------
    // Project state scanning
    // ------------------------------------------------------------------

    /// Create a temp directory with the full BMad output structure.
    fn make_bmad_project(dir: &std::path::Path) {
        use std::fs;
        fs::create_dir_all(dir.join("_bmad")).unwrap();
        fs::create_dir_all(dir.join("_bmad-output/planning-artifacts/epics")).unwrap();
        fs::create_dir_all(dir.join("_bmad-output/implementation-artifacts")).unwrap();

        fs::write(dir.join("_bmad-output/planning-artifacts/PRD.md"), "# PRD").unwrap();
        fs::write(
            dir.join("_bmad-output/planning-artifacts/architecture.md"),
            "# Arch",
        )
        .unwrap();
        fs::write(
            dir.join("_bmad-output/planning-artifacts/epics/epic-1.md"),
            "# Epic 1",
        )
        .unwrap();
        fs::write(
            dir.join("_bmad-output/planning-artifacts/epics/epic-2.md"),
            "# Epic 2",
        )
        .unwrap();
        fs::write(
            dir.join("_bmad-output/planning-artifacts/epics/epic-3.md"),
            "# Epic 3",
        )
        .unwrap();
        fs::write(
            dir.join("_bmad-output/implementation-artifacts/sprint-status.yaml"),
            "status: active",
        )
        .unwrap();
        fs::write(dir.join("_bmad-output/project-context.md"), "# Context").unwrap();
    }

    #[test]
    fn scan_project_full_structure() {
        let tmp = tempfile::tempdir().unwrap();
        make_bmad_project(tmp.path());

        let result = BmadIndex::scan_project_dir(tmp.path()).unwrap();
        assert!(result.bmad_installed);
        assert!(result.prd_found);
        assert!(result.architecture_found);
        assert_eq!(result.epic_count, 3);
        assert!(result.sprint_status_found);
        assert!(result.project_context_found);
    }

    #[test]
    fn scan_project_not_bmad() {
        let tmp = tempfile::tempdir().unwrap();
        // Empty directory — no _bmad/

        let result = BmadIndex::scan_project_dir(tmp.path()).unwrap();
        assert!(!result.bmad_installed);
        assert!(!result.prd_found);
        assert!(!result.architecture_found);
        assert_eq!(result.epic_count, 0);
        assert!(!result.sprint_status_found);
        assert!(!result.project_context_found);
    }

    #[test]
    fn scan_project_partial_artifacts() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("_bmad")).unwrap();
        std::fs::create_dir_all(tmp.path().join("_bmad-output/planning-artifacts/epics")).unwrap();
        std::fs::write(
            tmp.path().join("_bmad-output/planning-artifacts/PRD.md"),
            "# PRD",
        )
        .unwrap();

        let result = BmadIndex::scan_project_dir(tmp.path()).unwrap();
        assert!(result.bmad_installed);
        assert!(result.prd_found);
        assert!(!result.architecture_found);
        assert_eq!(result.epic_count, 0);
        assert!(!result.sprint_status_found);
        assert!(!result.project_context_found);
    }

    #[test]
    fn scan_project_epics_ignores_non_md() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("_bmad")).unwrap();
        std::fs::create_dir_all(tmp.path().join("_bmad-output/planning-artifacts/epics")).unwrap();
        std::fs::write(
            tmp.path().join("_bmad-output/planning-artifacts/epics/epic-1.md"),
            "# Epic",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("_bmad-output/planning-artifacts/epics/.DS_Store"),
            "",
        )
        .unwrap();
        std::fs::write(
            tmp.path()
                .join("_bmad-output/planning-artifacts/epics/notes.txt"),
            "notes",
        )
        .unwrap();

        let result = BmadIndex::scan_project_dir(tmp.path()).unwrap();
        assert_eq!(result.epic_count, 1, "should only count .md files");
    }

    #[test]
    fn scan_project_nonexistent_path() {
        let result = BmadIndex::scan_project_dir(std::path::Path::new("/nonexistent/path/xyz"));
        assert!(result.is_err(), "should return error for nonexistent path");
    }

    // ------------------------------------------------------------------
    // Doc validation
    // ------------------------------------------------------------------

    #[test]
    fn validate_docs_embedded_is_valid() {
        let result = BmadIndex::validate_docs(BMAD_DOCS);
        assert!(result.valid, "embedded docs should pass validation: {:?}", result.errors);
    }

    #[test]
    fn validate_docs_empty_string_is_invalid() {
        let result = BmadIndex::validate_docs("");
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn validate_docs_too_small_is_invalid() {
        let result = BmadIndex::validate_docs("tiny");
        assert!(!result.valid);
        let err_text = result.errors.join(" ");
        assert!(err_text.contains("too small"));
    }

    #[test]
    fn validate_docs_missing_phases_is_invalid() {
        // Has size but missing phase keywords
        let content = "x".repeat(600);
        let result = BmadIndex::validate_docs(&content);
        assert!(!result.valid);
        let err_text = result.errors.join(" ");
        assert!(err_text.contains("analysis"));
    }

    #[test]
    fn validate_docs_valid_content() {
        // Craft a minimal doc that has all required keywords
        let content = format!(
            "{}\nanalysis planning solutioning implementation prd architecture sprint epic",
            "x".repeat(600)
        );
        let result = BmadIndex::validate_docs(&content);
        assert!(result.valid, "should be valid: {:?}", result.errors);
    }

    #[test]
    fn build_with_invalid_docs_still_works() {
        // build_with_docs doesn't validate — it always builds.
        // The caller (refresh_docs) is responsible for calling validate_docs first.
        let idx = BmadIndex::build_with_docs("garbage".to_string());
        assert!(idx.all_workflow_ids().len() >= 18, "workflows come from static data, not docs");
        assert_eq!(idx.raw_docs(), "garbage");
    }

    // ------------------------------------------------------------------
    // Doc source & metadata
    // ------------------------------------------------------------------

    #[test]
    fn build_sets_source_and_metadata() {
        let idx = BmadIndex::build();
        assert_eq!(*idx.doc_source(), DocSource::Embedded);
        assert!(idx.last_refresh().is_some());
        assert!(idx.doc_byte_size() > 0);
    }

    #[test]
    fn build_with_source_tracks_url() {
        let idx = BmadIndex::build_with_source(
            "test content".to_string(),
            DocSource::Url("https://example.com".to_string()),
        );
        assert_eq!(*idx.doc_source(), DocSource::Url("https://example.com".to_string()));
        assert_eq!(idx.doc_byte_size(), 12);
    }

    #[test]
    fn doc_source_display() {
        assert_eq!(DocSource::Embedded.to_string(), "embedded");
        assert_eq!(
            DocSource::Url("https://example.com".to_string()).to_string(),
            "url: https://example.com"
        );
        assert_eq!(
            DocSource::Cache("/tmp/test".to_string()).to_string(),
            "cache: /tmp/test"
        );
    }

    // ------------------------------------------------------------------
    // Scaffold
    // ------------------------------------------------------------------

    #[test]
    fn scaffold_quick_flow_creates_expected_files() {
        let tmp = tempfile::tempdir().unwrap();
        let result = BmadIndex::scaffold_project(tmp.path(), Track::QuickFlow).unwrap();

        assert_eq!(result.track, Track::QuickFlow);
        assert!(result.files_created.iter().any(|f| f.contains("project-context.md")));
        assert!(result.files_created.iter().any(|f| f.contains("tech-spec.md")));
        // Quick Flow should NOT create PRD or architecture
        assert!(!result.files_created.iter().any(|f| f.contains("PRD.md")));
        assert!(!result.files_created.iter().any(|f| f.contains("architecture.md")));
        assert_eq!(result.files_created.len(), 2);
        assert!(result.next_steps.contains(&"bmad-quick-dev"));
    }

    #[test]
    fn scaffold_bmad_method_creates_expected_files() {
        let tmp = tempfile::tempdir().unwrap();
        let result = BmadIndex::scaffold_project(tmp.path(), Track::BmadMethod).unwrap();

        assert_eq!(result.track, Track::BmadMethod);
        assert!(result.files_created.iter().any(|f| f.contains("project-context.md")));
        assert!(result.files_created.iter().any(|f| f.contains("PRD.md")));
        assert!(result.files_created.iter().any(|f| f.contains("architecture.md")));
        assert!(result.files_created.iter().any(|f| f.contains("epic-1.md")));
        assert_eq!(result.files_created.len(), 4);
        assert!(result.next_steps.contains(&"bmad-create-prd"));
        assert!(result.next_steps.contains(&"bmad-create-architecture"));
    }

    #[test]
    fn scaffold_enterprise_creates_expected_files() {
        let tmp = tempfile::tempdir().unwrap();
        let result = BmadIndex::scaffold_project(tmp.path(), Track::Enterprise).unwrap();

        assert_eq!(result.track, Track::Enterprise);
        assert!(result.files_created.iter().any(|f| f.contains("project-context.md")));
        assert!(result.files_created.iter().any(|f| f.contains("PRD.md")));
        assert!(result.files_created.iter().any(|f| f.contains("architecture.md")));
        assert!(result.files_created.iter().any(|f| f.contains("epic-1.md")));
        assert!(result.files_created.iter().any(|f| f.contains("security.md")));
        assert!(result.files_created.iter().any(|f| f.contains("devops.md")));
        assert_eq!(result.files_created.len(), 6);
    }

    #[test]
    fn scaffold_creates_directory_structure() {
        let tmp = tempfile::tempdir().unwrap();
        BmadIndex::scaffold_project(tmp.path(), Track::BmadMethod).unwrap();

        assert!(tmp.path().join("_bmad").is_dir());
        assert!(tmp.path().join("_bmad-output/planning-artifacts").is_dir());
        assert!(tmp.path().join("_bmad-output/planning-artifacts/epics").is_dir());
        assert!(tmp.path().join("_bmad-output/implementation-artifacts").is_dir());
    }

    #[test]
    fn scaffold_files_contain_boilerplate() {
        let tmp = tempfile::tempdir().unwrap();
        BmadIndex::scaffold_project(tmp.path(), Track::BmadMethod).unwrap();

        let prd = std::fs::read_to_string(
            tmp.path().join("_bmad-output/planning-artifacts/PRD.md"),
        )
        .unwrap();
        assert!(prd.contains("Product Requirements Document"));
        assert!(prd.contains("TODO"));

        let arch = std::fs::read_to_string(
            tmp.path().join("_bmad-output/planning-artifacts/architecture.md"),
        )
        .unwrap();
        assert!(arch.contains("Architecture Document"));
        assert!(arch.contains("TODO"));

        let epic = std::fs::read_to_string(
            tmp.path().join("_bmad-output/planning-artifacts/epics/epic-1.md"),
        )
        .unwrap();
        assert!(epic.contains("Epic 1"));
        assert!(epic.contains("TODO"));

        let ctx = std::fs::read_to_string(
            tmp.path().join("_bmad-output/project-context.md"),
        )
        .unwrap();
        assert!(ctx.contains("Project Context"));
        assert!(ctx.contains("BMad Method"));
    }

    #[test]
    fn scaffold_enterprise_has_security_and_devops_content() {
        let tmp = tempfile::tempdir().unwrap();
        BmadIndex::scaffold_project(tmp.path(), Track::Enterprise).unwrap();

        let security = std::fs::read_to_string(
            tmp.path().join("_bmad-output/planning-artifacts/security.md"),
        )
        .unwrap();
        assert!(security.contains("Security"));
        assert!(security.contains("Threat Model"));

        let devops = std::fs::read_to_string(
            tmp.path().join("_bmad-output/planning-artifacts/devops.md"),
        )
        .unwrap();
        assert!(devops.contains("DevOps"));
        assert!(devops.contains("CI/CD"));
    }

    #[test]
    fn scaffold_enterprise_prd_has_compliance_section() {
        let tmp = tempfile::tempdir().unwrap();
        BmadIndex::scaffold_project(tmp.path(), Track::Enterprise).unwrap();

        let prd = std::fs::read_to_string(
            tmp.path().join("_bmad-output/planning-artifacts/PRD.md"),
        )
        .unwrap();
        assert!(prd.contains("Compliance"), "Enterprise PRD should have compliance section");
    }

    #[test]
    fn scaffold_enterprise_architecture_has_security_section() {
        let tmp = tempfile::tempdir().unwrap();
        BmadIndex::scaffold_project(tmp.path(), Track::Enterprise).unwrap();

        let arch = std::fs::read_to_string(
            tmp.path().join("_bmad-output/planning-artifacts/architecture.md"),
        )
        .unwrap();
        assert!(arch.contains("Security Architecture"),
            "Enterprise architecture should have security section");
    }

    #[test]
    fn scaffold_detected_by_scan_project_dir() {
        let tmp = tempfile::tempdir().unwrap();
        BmadIndex::scaffold_project(tmp.path(), Track::BmadMethod).unwrap();

        let state = BmadIndex::scan_project_dir(tmp.path()).unwrap();
        assert!(state.bmad_installed, "scaffolded project should have _bmad dir");
        assert!(state.prd_found, "scaffolded project should have PRD.md");
        assert!(state.architecture_found, "scaffolded project should have architecture.md");
        assert_eq!(state.epic_count, 1, "scaffolded project should have 1 epic");
        assert!(state.project_context_found, "scaffolded project should have project-context.md");
    }

    #[test]
    fn scaffold_nonexistent_dir_errors() {
        let result = BmadIndex::scaffold_project(
            std::path::Path::new("/nonexistent/scaffold/dir"),
            Track::BmadMethod,
        );
        assert!(result.is_err(), "should fail for nonexistent directory");
    }

    #[test]
    fn scaffold_quick_flow_context_mentions_track() {
        let tmp = tempfile::tempdir().unwrap();
        BmadIndex::scaffold_project(tmp.path(), Track::QuickFlow).unwrap();

        let ctx = std::fs::read_to_string(
            tmp.path().join("_bmad-output/project-context.md"),
        )
        .unwrap();
        assert!(ctx.contains("Quick Flow"), "Quick Flow context should mention the track");
    }
}
