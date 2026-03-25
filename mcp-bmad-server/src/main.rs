mod bmad_index;

use std::sync::Arc;

use bmad_index::{BmadIndex, Phase, Track};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Tool parameter types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetWorkflowRequest {
    /// The workflow skill id, e.g. "bmad-create-prd".
    #[schemars(description = "The workflow skill id, e.g. \"bmad-create-prd\"")]
    workflow_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetNextStepsRequest {
    /// The phase name: "Analysis", "Planning", "Solutioning", or "Implementation".
    #[schemars(description = "Phase name: Analysis, Planning, Solutioning, or Implementation")]
    phase: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetTrackWorkflowsRequest {
    /// The planning track: "Quick Flow", "BMad Method", or "Enterprise".
    #[schemars(description = "Planning track: Quick Flow, BMad Method, or Enterprise")]
    track: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NextStepRequest {
    /// JSON object or free-text description of current project state.
    /// E.g. "has PRD, has architecture, no epics yet"
    #[schemars(
        description = "JSON object or free-text description of current project state, e.g. \"has PRD, has architecture, no epics yet\""
    )]
    project_state: String,
    /// Optional: the last workflow completed.
    #[schemars(description = "The last workflow completed, e.g. \"bmad-create-prd\"")]
    last_workflow: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct HelpRequest {
    /// Natural language question about the BMad Method.
    #[schemars(
        description = "Natural language question about the BMad Method, e.g. \"what does the SM agent do?\""
    )]
    question: String,
    /// Optional: current project context (e.g. track, phase, what exists).
    #[schemars(
        description = "Optional current project context, e.g. \"BMad Method track, Planning phase\""
    )]
    context: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[allow(dead_code)]
struct CheckReadinessRequest {
    /// Describe what planning artifacts exist.
    /// E.g. "PRD.md done, architecture.md done, epics created, no sprint-status.yaml yet"
    #[schemars(
        description = "Describe what planning artifacts exist, e.g. \"PRD.md done, architecture.md done, epics created\""
    )]
    project_state: String,
    /// The planning track being used: Quick Flow, BMad Method, or Enterprise.
    /// Defaults to "BMad Method" if not specified.
    #[schemars(
        description = "Planning track: Quick Flow, BMad Method, or Enterprise. Defaults to BMad Method."
    )]
    track: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ListAgentsRequest {
    /// Optional filter: only return agents that handle workflows in this phase.
    #[schemars(
        description = "Optional phase filter: Analysis, Planning, Solutioning, or Implementation"
    )]
    phase: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AgentInfoRequest {
    /// The agent skill id, e.g. "bmad-pm".
    #[schemars(description = "The agent skill id, e.g. \"bmad-pm\" or \"bmad-architect\"")]
    agent_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SprintGuideRequest {
    /// Current sprint status. E.g. "epic 1 complete, working on epic 2 story 3,
    /// story file created but not yet implemented".
    #[schemars(
        description = "Current sprint status describing where you are in the build cycle. \
            E.g. \"epic 1 complete, working on epic 2 story 3, story file created but not yet implemented\""
    )]
    sprint_state: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct RefreshDocsRequest {}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct BmadServer {
    tool_router: ToolRouter<Self>,
    index: Arc<RwLock<BmadIndex>>,
}

fn parse_phase(s: &str) -> Option<Phase> {
    match s.to_lowercase().as_str() {
        "analysis" | "phase 1" | "1" => Some(Phase::Analysis),
        "planning" | "phase 2" | "2" => Some(Phase::Planning),
        "solutioning" | "phase 3" | "3" => Some(Phase::Solutioning),
        "implementation" | "phase 4" | "4" => Some(Phase::Implementation),
        _ => None,
    }
}

fn parse_track(s: &str) -> Option<Track> {
    match s.to_lowercase().replace('-', " ").as_str() {
        "quick flow" | "quickflow" | "quick" => Some(Track::QuickFlow),
        "bmad method" | "bmad" | "bmadmethod" => Some(Track::BmadMethod),
        "enterprise" => Some(Track::Enterprise),
        _ => None,
    }
}

/// Check whether a lowercased text contains any of the given keyword phrases.
fn has_keywords(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| text.contains(kw))
}

/// Default cache directory for fetched docs.
fn default_cache_path() -> std::path::PathBuf {
    let base = if let Some(home) = std::env::var_os("HOME") {
        std::path::PathBuf::from(home).join(".cache").join("bmad-mcp")
    } else {
        std::path::PathBuf::from("/tmp/bmad-mcp")
    };
    base.join("llms-full.txt")
}

/// Fetch docs from URL and cache to disk.
async fn fetch_and_cache_docs(url: &str) -> Result<String, String> {
    let cache_path = std::env::var("BMAD_DOCS_CACHE_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| default_cache_path());

    tracing::info!(%url, ?cache_path, "fetching docs from remote source");

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} from {}", response.status(), url));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("failed to read response body: {e}"))?;

    if body.trim().is_empty() {
        return Err("remote docs response was empty".to_string());
    }

    // Cache to disk
    if let Some(parent) = cache_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    if let Err(e) = tokio::fs::write(&cache_path, &body).await {
        tracing::warn!(?cache_path, %e, "failed to cache docs to disk");
    } else {
        tracing::info!(?cache_path, bytes = body.len(), "cached docs to disk");
    }

    Ok(body)
}

/// Load docs for startup: try URL first, then cache, then embedded fallback.
async fn load_startup_docs() -> String {
    let url = std::env::var("BMAD_DOCS_URL").ok();

    if let Some(ref url) = url {
        match fetch_and_cache_docs(url).await {
            Ok(docs) => {
                tracing::info!("using remote docs ({} bytes)", docs.len());
                return docs;
            }
            Err(e) => {
                tracing::warn!(%e, "failed to fetch remote docs, trying cache");
            }
        }
    }

    // Try loading from cache
    let cache_path = std::env::var("BMAD_DOCS_CACHE_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| default_cache_path());

    if let Ok(cached) = tokio::fs::read_to_string(&cache_path).await
        && !cached.trim().is_empty()
    {
        tracing::info!(?cache_path, "using cached docs ({} bytes)", cached.len());
        return cached;
    }

    tracing::info!("using embedded docs");
    BmadIndex::embedded_docs().to_string()
}

#[tool_router]
impl BmadServer {
    fn new(index: Arc<RwLock<BmadIndex>>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            index,
        }
    }

    #[tool(description = "Get metadata for a BMad Method workflow by its skill id. \
        Returns the workflow description, phase, agent, outputs, prerequisites, \
        next steps, and applicable planning tracks.")]
    async fn bmad_get_workflow(
        &self,
        Parameters(req): Parameters<GetWorkflowRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = self.index.read().await;
        match idx.get_workflow(&req.workflow_id) {
            Some(wf) => {
                let tracks: Vec<&str> = wf.tracks.iter().map(|t| t.name()).collect();
                let text = format!(
                    "## {id}\n\n\
                     **Description:** {desc}\n\
                     **Phase:** {phase} (Phase {num})\n\
                     **Agent:** {agent}\n\
                     **Produces:** {produces}\n\
                     **Prerequisites:** {prereqs}\n\
                     **Next steps:** {next}\n\
                     **Tracks:** {tracks}",
                    id = wf.id,
                    desc = wf.description,
                    phase = wf.phase.name(),
                    num = wf.phase.number(),
                    agent = wf.agent,
                    produces = wf.produces,
                    prereqs = if wf.prerequisites.is_empty() {
                        "none".to_string()
                    } else {
                        wf.prerequisites.join(", ")
                    },
                    next = if wf.next_steps.is_empty() {
                        "none".to_string()
                    } else {
                        wf.next_steps.join(", ")
                    },
                    tracks = tracks.join(", "),
                );
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            None => {
                let all = idx.all_workflow_ids();
                let text = format!(
                    "Workflow '{}' not found. Available workflows:\n{}",
                    req.workflow_id,
                    all.join(", ")
                );
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
        }
    }

    #[tool(description = "Get the recommended next-step workflows after completing a phase. \
        Returns an ordered list of workflow ids to proceed with.")]
    async fn bmad_get_next_steps(
        &self,
        Parameters(req): Parameters<GetNextStepsRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = self.index.read().await;
        match parse_phase(&req.phase) {
            Some(phase) => {
                let steps = idx.get_next_steps(phase);
                let text = format!(
                    "## Next steps after {name} (Phase {num})\n\n{list}",
                    name = phase.name(),
                    num = phase.number(),
                    list = steps
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            let desc = idx
                                .get_workflow(s)
                                .map(|w| w.description)
                                .unwrap_or("(unknown)");
                            format!("{}. `{}` — {}", i + 1, s, desc)
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            None => {
                let text = format!(
                    "Unknown phase '{}'. Valid phases: Analysis, Planning, Solutioning, Implementation",
                    req.phase
                );
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
        }
    }

    #[tool(description = "List all workflows for a given planning track \
        (Quick Flow, BMad Method, or Enterprise).")]
    async fn bmad_get_track_workflows(
        &self,
        Parameters(req): Parameters<GetTrackWorkflowsRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = self.index.read().await;
        match parse_track(&req.track) {
            Some(track) => {
                let wfs = idx.get_track_workflows(track);
                let mut lines = vec![format!("## {} Track Workflows\n", track.name())];
                for wf in &wfs {
                    lines.push(format!(
                        "- `{}` — {} (Phase {}: {})",
                        wf.id,
                        wf.description,
                        wf.phase.number(),
                        wf.phase.name(),
                    ));
                }
                Ok(CallToolResult::success(vec![Content::text(
                    lines.join("\n"),
                )]))
            }
            None => {
                let text = format!(
                    "Unknown track '{}'. Valid tracks: Quick Flow, BMad Method, Enterprise",
                    req.track
                );
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
        }
    }

    #[tool(description = "Recommend the next BMad Method workflow to run based on current project state. \
        Parses a free-text or JSON description of what artifacts exist (PRD, architecture, epics, etc.) \
        and returns the specific next workflow, which agent to use, and the command.")]
    async fn bmad_next_step(
        &self,
        Parameters(req): Parameters<NextStepRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = self.index.read().await;
        let completed = BmadIndex::infer_completed_workflows(&req.project_state);
        let phase = BmadIndex::infer_current_phase(&completed);

        let recommendations = idx.recommend_next(&completed, req.last_workflow.as_deref());

        if recommendations.is_empty() {
            let text = format!(
                "## Next Step\n\n\
                 Based on your project state, you appear to be in the **{phase}** phase \
                 (Phase {num}).\n\n\
                 All workflows in this phase appear to be completed. \
                 Consider running `bmad-retrospective` to review progress, \
                 or check if there are remaining stories to implement.",
                phase = phase.name(),
                num = phase.number(),
            );
            return Ok(CallToolResult::success(vec![Content::text(text)]));
        }

        let completed_list = if completed.is_empty() {
            "none detected".to_string()
        } else {
            completed
                .iter()
                .map(|id| format!("`{id}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let mut lines = vec![
            format!(
                "## Next Step Recommendation\n\n\
                 **Current phase:** {} (Phase {})\n\
                 **Detected completed artifacts:** {}\n",
                phase.name(),
                phase.number(),
                completed_list,
            ),
            "### Recommended next workflow(s):\n".to_string(),
        ];

        for (i, wf) in recommendations.iter().enumerate() {
            lines.push(format!(
                "{}. **`{}`** — {}\n   \
                 - Agent: `{}` (invoke this agent to run the workflow)\n   \
                 - Produces: {}\n   \
                 - Prerequisites: {}",
                i + 1,
                wf.id,
                wf.description,
                wf.agent,
                wf.produces,
                if wf.prerequisites.is_empty() {
                    "none".to_string()
                } else {
                    wf.prerequisites.join(", ")
                },
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    #[tool(description = "Answer questions about the BMad Method — phases, agents, workflows, \
        tracks, and core tools. Searches the BMad documentation and structured index \
        to provide contextual answers.")]
    async fn bmad_help(
        &self,
        Parameters(req): Parameters<HelpRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = self.index.read().await;
        let query = &req.question;

        // Build context-aware search: combine question + optional context
        let search_term = if let Some(ref ctx) = req.context {
            format!("{query} {ctx}")
        } else {
            query.clone()
        };

        let results = idx.search(&search_term);

        let mut lines = vec![format!("## BMad Method Help\n\n**Question:** {query}\n")];

        if let Some(ref ctx) = req.context {
            lines.push(format!("**Context:** {ctx}\n"));
        }

        if results.is_empty() {
            // Fall back to raw docs search
            let docs = idx.raw_docs();
            let lower = search_term.to_lowercase();
            let relevant: Vec<&str> = docs
                .lines()
                .filter(|line| line.to_lowercase().contains(&lower))
                .take(10)
                .collect();

            if relevant.is_empty() {
                lines.push(
                    "No specific results found. Try asking about:\n\
                     - A specific agent (e.g. \"SM agent\", \"Analyst\")\n\
                     - A workflow (e.g. \"create-prd\", \"sprint-planning\")\n\
                     - A phase (e.g. \"Analysis\", \"Planning\")\n\
                     - A track (e.g. \"Quick Flow\", \"Enterprise\")"
                        .to_string(),
                );
            } else {
                lines.push("### From BMad Documentation\n".to_string());
                for line in relevant {
                    lines.push(format!("  {line}"));
                }
            }
        } else {
            lines.push(format!("### Results ({})\n", results.len()));
            for result in &results {
                lines.push(result.clone());
                lines.push(String::new());
            }
        }

        // Add overview for common question patterns
        let lower_q = query.to_lowercase();
        if lower_q.contains("phase") || lower_q.contains("overview") || lower_q.contains("how") {
            lines.push("### Phase Overview\n".to_string());
            for phase in Phase::all() {
                let wf_count = idx.get_phase_workflows(*phase).len();
                lines.push(format!(
                    "- **Phase {}: {}** — {} ({} workflows)",
                    phase.number(),
                    phase.name(),
                    phase.description(),
                    wf_count,
                ));
            }
        }

        if lower_q.contains("track") {
            lines.push("\n### Track Overview\n".to_string());
            for track in Track::all() {
                lines.push(format!(
                    "- **{}** — {}",
                    track.name(),
                    track.description(),
                ));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    #[tool(description = "Check whether a project is ready to enter the Implementation phase. \
        Validates that all required planning artifacts exist for the given track \
        (Quick Flow, BMad Method, or Enterprise). Returns readiness status, \
        missing artifacts, warnings, and recommended next action.")]
    async fn bmad_check_readiness(
        &self,
        Parameters(req): Parameters<CheckReadinessRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let track = req
            .track
            .as_deref()
            .and_then(parse_track)
            .unwrap_or(Track::BmadMethod);

        let result = BmadIndex::check_readiness(&req.project_state, track);

        let status = if result.ready { "READY" } else { "NOT READY" };
        let mut lines = vec![format!(
            "## Implementation Readiness: {status}\n\n\
             **Track:** {track}\n\
             **Project state:** {state}\n",
            track = track.name(),
            state = req.project_state,
        )];

        if !result.missing_artifacts.is_empty() {
            lines.push("### Missing Artifacts (required)\n".to_string());
            for artifact in &result.missing_artifacts {
                lines.push(format!("- {artifact}"));
            }
            lines.push(String::new());
        }

        if !result.warnings.is_empty() {
            lines.push("### Warnings\n".to_string());
            for warning in &result.warnings {
                lines.push(format!("- {warning}"));
            }
            lines.push(String::new());
        }

        lines.push(format!("### Next Action\n\n{}", result.next_action));

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    #[tool(description = "List all BMad Method agents. Optionally filter by phase to show only \
        agents that handle workflows in that phase. Returns each agent's skill id, role, \
        persona name, and primary workflows.")]
    async fn bmad_list_agents(
        &self,
        Parameters(req): Parameters<ListAgentsRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = self.index.read().await;

        let agents = if let Some(ref phase_str) = req.phase {
            match parse_phase(phase_str) {
                Some(phase) => idx.get_agents_by_phase(phase),
                None => {
                    let text = format!(
                        "Unknown phase '{}'. Valid phases: Analysis, Planning, Solutioning, Implementation",
                        phase_str
                    );
                    return Ok(CallToolResult::success(vec![Content::text(text)]));
                }
            }
        } else {
            idx.all_agents()
        };

        let header = if let Some(ref phase_str) = req.phase {
            format!("## BMad Agents — {} Phase\n", phase_str)
        } else {
            "## BMad Agents\n".to_string()
        };

        let mut lines = vec![header];

        if agents.is_empty() {
            lines.push("No agents found for the given filter.".to_string());
        } else {
            for agent in &agents {
                let workflows = agent.primary_workflows.join(", ");
                lines.push(format!(
                    "### `{skill_id}` — {name} (persona: {persona})\n\
                     - **Workflows:** {workflows}\n",
                    skill_id = agent.skill_id,
                    name = agent.name,
                    persona = agent.persona,
                ));
            }
            lines.push(format!("*{} agent(s) total*", agents.len()));
        }

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    #[tool(description = "Get detailed information about a specific BMad Method agent by skill id. \
        Returns the agent's full profile: name, persona, skill id, and all primary workflows \
        with their descriptions.")]
    async fn bmad_agent_info(
        &self,
        Parameters(req): Parameters<AgentInfoRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = self.index.read().await;
        match idx.get_agent(&req.agent_id) {
            Some(agent) => {
                let mut lines = vec![format!(
                    "## Agent: {name}\n\n\
                     **Skill ID:** `{skill_id}`\n\
                     **Persona:** {persona}\n\
                     **Role:** {name}\n\n\
                     ### Primary Workflows\n",
                    name = agent.name,
                    skill_id = agent.skill_id,
                    persona = agent.persona,
                )];

                for wf_id in &agent.primary_workflows {
                    if let Some(wf) = idx.get_workflow(wf_id) {
                        let tracks: Vec<&str> = wf.tracks.iter().map(|t| t.name()).collect();
                        lines.push(format!(
                            "- `{id}` — {desc}\n  Phase: {phase} | Produces: {produces} | Tracks: {tracks}",
                            id = wf.id,
                            desc = wf.description,
                            phase = wf.phase.name(),
                            produces = wf.produces,
                            tracks = tracks.join(", "),
                        ));
                    } else {
                        lines.push(format!("- `{wf_id}` — (workflow not found in index)"));
                    }
                }

                lines.push(format!(
                    "\n### How to invoke\n\n\
                     Use the agent skill `{}` to start any of the above workflows. \
                     The agent will guide you through the process.",
                    agent.skill_id,
                ));

                Ok(CallToolResult::success(vec![Content::text(
                    lines.join("\n"),
                )]))
            }
            None => {
                let all: Vec<&str> = idx.all_agents().iter().map(|a| a.skill_id).collect();
                let text = format!(
                    "Agent '{}' not found. Available agents:\n{}",
                    req.agent_id,
                    all.join(", ")
                );
                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
        }
    }

    #[tool(description = "Guide an AI agent through the Implementation phase build cycle. \
        The BMad build cycle is: SM creates story -> DEV implements -> DEV reviews. \
        This repeats per story; after all stories in an epic, SM runs retrospective. \
        Given your current sprint state, returns which step you are on, which agent \
        and workflow to invoke next, and what comes after.")]
    async fn bmad_sprint_guide(
        &self,
        Parameters(req): Parameters<SprintGuideRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let state = req.sprint_state.to_lowercase();

        // Determine cycle state from keywords in the sprint_state description.
        // Priority order matters: check most specific conditions first.

        let (current_step, agent_to_invoke, workflow_to_run, rationale, after_this) =
            if has_keywords(&state, &["no sprint", "no plan", "not started", "beginning", "brand new", "just started implementation"]) {
                (
                    "Sprint initialization",
                    "bmad-agent-sm",
                    "bmad-sprint-planning",
                    "No sprint plan detected. The Scrum Master needs to initialize sprint tracking \
                     (sprint-status.yaml) before stories can be created.",
                    "Once sprint planning is complete, the SM will create the first story file \
                     for the first epic.",
                )
            } else if has_keywords(&state, &["all stories in epic done", "all stories done", "all stories complete",
                "epic complete", "epic done", "epic finished", "stories in epic complete",
                "all stories reviewed", "last story reviewed", "last story done",
                "entire epic implemented and reviewed"]) {
                (
                    "Epic retrospective",
                    "bmad-agent-sm",
                    "bmad-retrospective",
                    "All stories in the current epic are complete. The Scrum Master should run a \
                     retrospective to review what went well, what didn't, and capture lessons \
                     before moving to the next epic.",
                    "After the retrospective, if more epics remain, the SM will create the first \
                     story file for the next epic. If all epics are done, the project is complete.",
                )
            } else if has_keywords(&state, &["implemented", "built", "coded", "developed", "implementation done",
                "implementation complete", "code complete", "code done"])
                && !has_keywords(&state, &["reviewed", "review done", "review complete", "passed review",
                    "code review done", "code review complete"]) {
                (
                    "Code review",
                    "bmad-agent-dev",
                    "bmad-code-review",
                    "The story has been implemented but not yet reviewed. The Developer should \
                     run the code review workflow to validate the implementation quality, check \
                     for edge cases, and ensure the story acceptance criteria are met.",
                    "After the code review passes, if more stories remain in the current epic, \
                     the SM will create the next story file. If this was the last story in the \
                     epic, the SM will run a retrospective.",
                )
            } else if has_keywords(&state, &["story file created", "story created", "story file exists",
                "story ready", "story prepared", "story written", "has story file",
                "story file done", "story defined"])
                && !has_keywords(&state, &["implemented", "built", "coded", "developed",
                    "implementation done", "code complete"]) {
                (
                    "Story implementation",
                    "bmad-agent-dev",
                    "bmad-dev-story",
                    "A story file has been created but the story has not been implemented yet. \
                     The Developer should implement the story according to its acceptance criteria \
                     and technical requirements.",
                    "After implementation, the Developer will run a code review on the completed \
                     story.",
                )
            } else if has_keywords(&state, &["reviewed", "review done", "review complete", "passed review",
                "code review done", "code review complete"])
                && has_keywords(&state, &["more stories", "stories remain", "next story", "remaining stories",
                    "not all stories"]) {
                (
                    "Create next story",
                    "bmad-agent-sm",
                    "bmad-create-story",
                    "The current story has been reviewed and more stories remain in the epic. \
                     The Scrum Master should create the next story file to continue the cycle.",
                    "After the story file is created, the Developer will implement it, then \
                     review it. This cycle continues until all stories in the epic are done.",
                )
            } else if has_keywords(&state, &["no story", "need story", "no current story",
                "story not created", "need to create story", "waiting for story"]) {
                (
                    "Create story",
                    "bmad-agent-sm",
                    "bmad-create-story",
                    "No current story file exists. The Scrum Master needs to create the next \
                     story file from the epic's story list so the Developer can implement it.",
                    "After the story file is created, the Developer will implement the story, \
                     then run a code review.",
                )
            } else if has_keywords(&state, &["retrospective done", "retro done", "retro complete",
                "retrospective complete"]) {
                (
                    "Start next epic",
                    "bmad-agent-sm",
                    "bmad-create-story",
                    "The retrospective for the previous epic is complete. The Scrum Master should \
                     create the first story file for the next epic to begin the build cycle again.",
                    "After the story file is created, the Developer will implement it. The \
                     SM creates story -> DEV implements -> DEV reviews cycle continues for \
                     each story in the new epic.",
                )
            } else {
                // Fallback: try to give reasonable guidance based on any partial signals
                if has_keywords(&state, &["sprint plan", "sprint status", "sprint-status"]) {
                    (
                        "Create story",
                        "bmad-agent-sm",
                        "bmad-create-story",
                        "A sprint plan exists. The Scrum Master should create the next story file \
                         so the Developer can begin implementation.",
                        "After the story file is created, the Developer will implement it, \
                         then run a code review.",
                    )
                } else {
                    (
                        "Sprint initialization",
                        "bmad-agent-sm",
                        "bmad-sprint-planning",
                        "Could not determine the exact cycle state from the description provided. \
                         Starting from the beginning: the Scrum Master should initialize sprint \
                         tracking. Provide more detail about your sprint state for more specific guidance.",
                        "Once sprint planning is complete, the SM will create story files and the \
                         DEV will implement and review them in sequence.",
                    )
                }
            };

        let text = format!(
            "## Sprint Guide\n\n\
             **Current step:** {current_step}\n\
             **Agent to invoke:** `{agent_to_invoke}`\n\
             **Workflow to run:** `{workflow_to_run}`\n\n\
             ### Rationale\n\n\
             {rationale}\n\n\
             ### After this step\n\n\
             {after_this}\n\n\
             ### Build cycle reference\n\n\
             1. **SM** creates story (`bmad-create-story` via `bmad-agent-sm`)\n\
             2. **DEV** implements story (`bmad-dev-story` via `bmad-agent-dev`)\n\
             3. **DEV** reviews story (`bmad-code-review` via `bmad-agent-dev`)\n\
             4. Repeat 1-3 for each story in the epic\n\
             5. **SM** runs retrospective (`bmad-retrospective` via `bmad-agent-sm`)\n\
             6. Repeat for next epic",
        );

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Refresh the BMad Method documentation cache from the remote source. \
        Requires BMAD_ALLOW_REFRESH=1 and BMAD_DOCS_URL to be set. \
        Fetches the latest docs, updates the cache, and rebuilds the index.")]
    async fn bmad_refresh_docs(
        &self,
        #[allow(unused_variables)] Parameters(_req): Parameters<RefreshDocsRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let allowed = std::env::var("BMAD_ALLOW_REFRESH")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if !allowed {
            return Ok(CallToolResult::success(vec![Content::text(
                "Doc refresh is disabled. Set BMAD_ALLOW_REFRESH=1 to enable.",
            )]));
        }

        let url = std::env::var("BMAD_DOCS_URL").map_err(|_| {
            rmcp::ErrorData::invalid_params(
                "BMAD_DOCS_URL is not set. Cannot refresh docs without a source URL.",
                None,
            )
        })?;

        tracing::info!(%url, "refreshing docs from remote source");

        let docs = fetch_and_cache_docs(&url).await.map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to fetch docs: {e}"), None)
        })?;

        let bytes = docs.len();
        let new_index = BmadIndex::build_with_docs(docs);
        let mut idx = self.index.write().await;
        *idx = new_index;

        tracing::info!(bytes, "index rebuilt with refreshed docs");

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Documentation refreshed successfully ({bytes} bytes). Index rebuilt.",
        ))]))
    }
}

#[tool_handler]
impl ServerHandler for BmadServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "BMad Method MCP Server",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "BMad Method MCP Server provides workflow guidance for the Build More Architect \
                 Dreams methodology. Use the tools to look up workflows, determine next steps \
                 for each phase, and find the right planning track for your project. Use \
                 bmad_next_step with a description of your project state to get a personalized \
                 recommendation, or bmad_help to ask questions about agents, phases, tracks, \
                 and workflows. The server knows about all phases (Analysis, Planning, \
                 Solutioning, Implementation), all three planning tracks (Quick Flow, BMad \
                 Method, Enterprise), and all default agents and their workflows."
                    .to_string(),
            )
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting BMad Method MCP server");

    let docs = load_startup_docs().await;
    let index = Arc::new(RwLock::new(BmadIndex::build_with_docs(docs)));
    tracing::info!("index built (lazy singleton, reused across all tool calls)");

    let server = BmadServer::new(index);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
