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

/// The in-memory index of all BMad Method content.
pub struct BmadIndex {
    workflows: HashMap<&'static str, Workflow>,
    agents: HashMap<&'static str, Agent>,
    core_tools: Vec<CoreTool>,
    phase_workflows: HashMap<Phase, Vec<&'static str>>,
}

impl BmadIndex {
    /// Build the index. This is called once at startup.
    pub fn build() -> Self {
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

        Self {
            workflows,
            agents,
            core_tools,
            phase_workflows,
        }
    }

    /// Return the raw embedded documentation.
    pub fn raw_docs() -> &'static str {
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
        self.agents.values().collect()
    }

    // -- Core tools --

    pub fn core_tools(&self) -> &[CoreTool] {
        &self.core_tools
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
        let docs = BmadIndex::raw_docs();
        assert!(docs.len() > 1000, "embedded docs should be > 1000 chars");
        assert!(docs.contains("BMad Method"));
    }
}
