mod bmad_index;

use std::sync::Arc;

use bmad_index::{BmadIndex, DocSource, Phase, ScaffoldResult, SprintGuideResult, TemplateVars, Track, WorkflowStep};
use rmcp::{
    RoleServer, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ListResourceTemplatesResult,
        ListResourcesResult, PaginatedRequestParams, RawResource, RawResourceTemplate,
        ReadResourceRequestParams, ReadResourceResult, ResourceContents,
        ResourceUpdatedNotificationParam, ServerCapabilities, ServerInfo,
        SubscribeRequestParams, UnsubscribeRequestParams,
        AnnotateAble,
    },
    schemars, tool, tool_handler, tool_router,
    service::RequestContext,
    transport::{
        stdio,
        streamable_http_server::{
            StreamableHttpServerConfig, StreamableHttpService,
            session::local::LocalSessionManager,
        },
    },
};
use serde::Serialize;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Output format helper
// ---------------------------------------------------------------------------

/// Return true when the caller requested JSON output.
fn wants_json(format: &Option<String>) -> bool {
    matches!(format.as_deref(), Some("json"))
}

/// Build a `CallToolResult` containing either pretty-printed JSON or markdown.
fn format_output<T: Serialize>(format: &Option<String>, json_val: &T, markdown: String) -> CallToolResult {
    if wants_json(format) {
        let json = serde_json::to_string_pretty(json_val).unwrap_or_else(|e| {
            format!("{{\"error\": \"serialization failed: {e}\"}}")
        });
        CallToolResult::success(vec![Content::text(json)])
    } else {
        CallToolResult::success(vec![Content::text(markdown)])
    }
}

// ---------------------------------------------------------------------------
// JSON response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct WorkflowResponse {
    id: String,
    description: String,
    phase: String,
    phase_number: u8,
    agent: String,
    produces: String,
    prerequisites: Vec<String>,
    next_steps: Vec<String>,
    tracks: Vec<String>,
}

#[derive(Serialize)]
struct WorkflowNotFoundResponse {
    error: String,
    available_workflows: Vec<String>,
}

#[derive(Serialize)]
struct NextStepsResponse {
    phase: String,
    phase_number: u8,
    steps: Vec<NextStepEntry>,
}

#[derive(Serialize)]
struct NextStepEntry {
    workflow_id: String,
    description: String,
}

#[derive(Serialize)]
struct PhaseErrorResponse {
    error: String,
    valid_phases: Vec<String>,
}

#[derive(Serialize)]
struct TrackWorkflowsResponse {
    track: String,
    workflows: Vec<TrackWorkflowEntry>,
}

#[derive(Serialize)]
struct TrackWorkflowEntry {
    id: String,
    description: String,
    phase: String,
    phase_number: u8,
}

#[derive(Serialize)]
struct TrackErrorResponse {
    error: String,
    valid_tracks: Vec<String>,
}

#[derive(Serialize)]
struct NextStepRecommendation {
    current_phase: String,
    current_phase_number: u8,
    detected_completed: Vec<String>,
    recommendations: Vec<RecommendationEntry>,
}

#[derive(Serialize)]
struct RecommendationEntry {
    workflow_id: String,
    description: String,
    agent: String,
    produces: String,
    prerequisites: Vec<String>,
}

#[derive(Serialize)]
struct HelpResponse {
    question: String,
    context: Option<String>,
    results: Vec<String>,
    phase_overview: Option<Vec<PhaseOverviewEntry>>,
    track_overview: Option<Vec<TrackOverviewEntry>>,
}

#[derive(Serialize)]
struct PhaseOverviewEntry {
    number: u8,
    name: String,
    description: String,
    workflow_count: usize,
}

#[derive(Serialize)]
struct TrackOverviewEntry {
    name: String,
    description: String,
}

#[derive(Serialize)]
struct ReadinessResponse {
    ready: bool,
    track: String,
    project_state: String,
    missing_artifacts: Vec<String>,
    warnings: Vec<String>,
    next_action: String,
}

#[derive(Serialize)]
struct AgentListResponse {
    phase_filter: Option<String>,
    agents: Vec<AgentEntry>,
    total: usize,
}

#[derive(Serialize)]
struct AgentEntry {
    skill_id: String,
    name: String,
    persona: String,
    workflows: Vec<String>,
}

#[derive(Serialize)]
struct AgentDetailResponse {
    skill_id: String,
    name: String,
    persona: String,
    workflows: Vec<AgentWorkflowEntry>,
}

#[derive(Serialize)]
struct AgentWorkflowEntry {
    id: String,
    description: String,
    phase: String,
    produces: String,
    tracks: Vec<String>,
}

#[derive(Serialize)]
struct AgentNotFoundResponse {
    error: String,
    available_agents: Vec<String>,
}

#[derive(Serialize)]
struct SprintGuideResponse {
    current_step: String,
    agent_to_invoke: String,
    workflow_to_run: String,
    rationale: String,
    after_this: String,
}

#[derive(Serialize)]
struct ProjectStateResponse {
    project_path: String,
    bmad_installed: bool,
    prd_found: bool,
    architecture_found: bool,
    epic_count: usize,
    sprint_status_found: bool,
    project_context_found: bool,
    suggested_track: String,
    current_phase: String,
    current_phase_number: u8,
    recommended_next_step: String,
}

#[derive(Serialize)]
struct IndexStatusResponse {
    doc_source: String,
    last_refresh: String,
    total_workflows: usize,
    total_agents: usize,
    doc_byte_size: usize,
}

#[derive(Serialize)]
struct RefreshResponse {
    status: String,
    message: String,
    bytes: Option<usize>,
}

#[derive(Serialize)]
struct ScaffoldResponse {
    track: String,
    project_dir: String,
    files_created: Vec<String>,
    next_steps: Vec<String>,
}

#[derive(Serialize)]
struct RunWorkflowResponse {
    workflow_id: String,
    action: String,
    current_step: usize,
    total_steps: usize,
    completed: bool,
    step: Option<RunWorkflowStepInfo>,
    next_workflow: Option<String>,
    message: String,
}

#[derive(Serialize)]
struct RunWorkflowStepInfo {
    number: usize,
    title: String,
    instructions: String,
    expected_output: String,
    agent_guidance: String,
}

// ---------------------------------------------------------------------------
// Tool parameter types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetWorkflowRequest {
    /// The workflow skill id, e.g. "bmad-create-prd".
    #[schemars(description = "The workflow skill id, e.g. \"bmad-create-prd\"")]
    workflow_id: String,
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetNextStepsRequest {
    /// The phase name: "Analysis", "Planning", "Solutioning", or "Implementation".
    #[schemars(description = "Phase name: Analysis, Planning, Solutioning, or Implementation")]
    phase: String,
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetTrackWorkflowsRequest {
    /// The planning track: "Quick Flow", "BMad Method", or "Enterprise".
    #[schemars(description = "Planning track: Quick Flow, BMad Method, or Enterprise")]
    track: String,
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
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
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
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
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
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
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ListAgentsRequest {
    /// Optional filter: only return agents that handle workflows in this phase.
    #[schemars(
        description = "Optional phase filter: Analysis, Planning, Solutioning, or Implementation"
    )]
    phase: Option<String>,
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AgentInfoRequest {
    /// The agent skill id, e.g. "bmad-pm".
    #[schemars(description = "The agent skill id, e.g. \"bmad-pm\" or \"bmad-architect\"")]
    agent_id: String,
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
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
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct RefreshDocsRequest {
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct IndexStatusRequest {
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[allow(dead_code)]
struct ProjectStateRequest {
    /// Absolute or relative path to the project root directory.
    #[schemars(
        description = "Absolute or relative path to the project root directory to scan for BMad artifacts"
    )]
    project_path: String,
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[allow(dead_code)]
struct ScaffoldRequest {
    /// The planning track: "quick_flow", "bmad_method", or "enterprise".
    #[schemars(
        description = "Planning track: quick_flow, bmad_method, or enterprise"
    )]
    track: String,
    /// Optional: absolute or relative path to the project directory.
    /// Defaults to the current working directory.
    #[schemars(
        description = "Absolute or relative path to the project root directory. Defaults to current directory."
    )]
    project_dir: Option<String>,
    /// Optional project name for template variable substitution.
    #[schemars(description = "Project name substituted into templates as {{project_name}}")]
    project_name: Option<String>,
    /// Optional author name for template variable substitution.
    #[schemars(description = "Author name substituted into templates as {{author}}")]
    author: Option<String>,
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[allow(dead_code)]
struct RunWorkflowRequest {
    /// Action to perform: "start", "next", or "status".
    #[schemars(
        description = "Action: \"start\" to begin a workflow, \"next\" to advance to the next step, \"status\" to check progress"
    )]
    action: String,
    /// The workflow skill id (required for "start"). E.g. "bmad-create-prd".
    #[schemars(
        description = "Workflow skill id, e.g. \"bmad-create-prd\". Required for action=start."
    )]
    workflow_id: Option<String>,
    /// Project directory path. Required for all actions.
    #[schemars(description = "Absolute or relative path to the project root directory.")]
    project_dir: String,
    /// Optional result/notes from completing the current step (used with action=next).
    #[schemars(
        description = "Optional notes from the completed step, used with action=next."
    )]
    step_result: Option<String>,
    /// Output format: "markdown" (default) or "json".
    #[schemars(description = "Output format: \"markdown\" (default) or \"json\"")]
    output_format: Option<String>,
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

type Subscriptions = Arc<RwLock<Vec<(String, rmcp::Peer<RoleServer>)>>>;

#[derive(Clone)]
struct BmadServer {
    tool_router: ToolRouter<Self>,
    index: Arc<RwLock<BmadIndex>>,
    /// URIs that clients have subscribed to for update notifications,
    /// paired with the peer that subscribed (for sending notifications).
    subscriptions: Subscriptions,
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
    match s.to_lowercase().replace(['-', '_'], " ").as_str() {
        "quick flow" | "quickflow" | "quick" => Some(Track::QuickFlow),
        "bmad method" | "bmad" | "bmadmethod" => Some(Track::BmadMethod),
        "enterprise" => Some(Track::Enterprise),
        _ => None,
    }
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

/// Fetch docs from URL with retry logic (max 3 attempts, exponential backoff starting at 1s), then cache to disk.
async fn fetch_and_cache_docs(url: &str) -> Result<String, String> {
    let cache_path = std::env::var("BMAD_DOCS_CACHE_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| default_cache_path());

    let max_attempts = 3u32;
    let mut last_err = String::new();

    for attempt in 1..=max_attempts {
        tracing::info!(%url, attempt, max_attempts, ?cache_path, "fetching docs from remote source");

        match try_fetch_docs(url).await {
            Ok(body) => {
                tracing::info!(%url, attempt, bytes = body.len(), "fetch succeeded");

                // Cache to disk
                if let Some(parent) = cache_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                if let Err(e) = tokio::fs::write(&cache_path, &body).await {
                    tracing::warn!(?cache_path, %e, "failed to cache docs to disk");
                } else {
                    tracing::info!(?cache_path, bytes = body.len(), "cached docs to disk");
                }

                return Ok(body);
            }
            Err(e) => {
                last_err = e;
                if attempt < max_attempts {
                    let delay = std::time::Duration::from_secs(1 << (attempt - 1));
                    tracing::warn!(%url, attempt, ?delay, error = %last_err, "fetch failed, retrying");
                    tokio::time::sleep(delay).await;
                } else {
                    tracing::error!(%url, attempt, error = %last_err, "fetch failed, no retries left");
                }
            }
        }
    }

    Err(last_err)
}

/// Single HTTP fetch attempt — returns body or error.
async fn try_fetch_docs(url: &str) -> Result<String, String> {
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

    Ok(body)
}

/// Load docs for startup: try URL first, then cache, then embedded fallback.
/// Returns `(doc_content, doc_source)`.
async fn load_startup_docs() -> (String, DocSource) {
    let url = std::env::var("BMAD_DOCS_URL").ok();

    if let Some(ref url) = url {
        match fetch_and_cache_docs(url).await {
            Ok(docs) => {
                tracing::info!("using remote docs ({} bytes)", docs.len());
                return (docs, DocSource::Url(url.clone()));
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
        return (
            cached,
            DocSource::Cache(cache_path.to_string_lossy().to_string()),
        );
    }

    tracing::info!("using embedded docs");
    (BmadIndex::embedded_docs().to_string(), DocSource::Embedded)
}

#[tool_router]
impl BmadServer {
    fn new(index: Arc<RwLock<BmadIndex>>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            index,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
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
                let json_resp = WorkflowResponse {
                    id: wf.id.to_string(),
                    description: wf.description.to_string(),
                    phase: wf.phase.name().to_string(),
                    phase_number: wf.phase.number(),
                    agent: wf.agent.to_string(),
                    produces: wf.produces.to_string(),
                    prerequisites: wf.prerequisites.iter().map(|s| s.to_string()).collect(),
                    next_steps: wf.next_steps.iter().map(|s| s.to_string()).collect(),
                    tracks: tracks.iter().map(|s| s.to_string()).collect(),
                };
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
                Ok(format_output(&req.output_format, &json_resp, text))
            }
            None => {
                let all = idx.all_workflow_ids();
                let json_resp = WorkflowNotFoundResponse {
                    error: format!("Workflow '{}' not found", req.workflow_id),
                    available_workflows: all.iter().map(|s| s.to_string()).collect(),
                };
                let text = format!(
                    "Workflow '{}' not found. Available workflows:\n{}",
                    req.workflow_id,
                    all.join(", ")
                );
                Ok(format_output(&req.output_format, &json_resp, text))
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
                let json_resp = NextStepsResponse {
                    phase: phase.name().to_string(),
                    phase_number: phase.number(),
                    steps: steps.iter().map(|s| NextStepEntry {
                        workflow_id: s.to_string(),
                        description: idx.get_workflow(s).map(|w| w.description).unwrap_or("(unknown)").to_string(),
                    }).collect(),
                };
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
                Ok(format_output(&req.output_format, &json_resp, text))
            }
            None => {
                let json_resp = PhaseErrorResponse {
                    error: format!("Unknown phase '{}'", req.phase),
                    valid_phases: vec!["Analysis".to_string(), "Planning".to_string(), "Solutioning".to_string(), "Implementation".to_string()],
                };
                let text = format!(
                    "Unknown phase '{}'. Valid phases: Analysis, Planning, Solutioning, Implementation",
                    req.phase
                );
                Ok(format_output(&req.output_format, &json_resp, text))
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
                let json_resp = TrackWorkflowsResponse {
                    track: track.name().to_string(),
                    workflows: wfs.iter().map(|wf| TrackWorkflowEntry {
                        id: wf.id.to_string(),
                        description: wf.description.to_string(),
                        phase: wf.phase.name().to_string(),
                        phase_number: wf.phase.number(),
                    }).collect(),
                };
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
                Ok(format_output(&req.output_format, &json_resp, lines.join("\n")))
            }
            None => {
                let json_resp = TrackErrorResponse {
                    error: format!("Unknown track '{}'", req.track),
                    valid_tracks: vec!["Quick Flow".to_string(), "BMad Method".to_string(), "Enterprise".to_string()],
                };
                let text = format!(
                    "Unknown track '{}'. Valid tracks: Quick Flow, BMad Method, Enterprise",
                    req.track
                );
                Ok(format_output(&req.output_format, &json_resp, text))
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
            let json_resp = NextStepRecommendation {
                current_phase: phase.name().to_string(),
                current_phase_number: phase.number(),
                detected_completed: completed.iter().map(|s| s.to_string()).collect(),
                recommendations: vec![],
            };
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
            return Ok(format_output(&req.output_format, &json_resp, text));
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

        let json_resp = NextStepRecommendation {
            current_phase: phase.name().to_string(),
            current_phase_number: phase.number(),
            detected_completed: completed.iter().map(|s| s.to_string()).collect(),
            recommendations: recommendations.iter().map(|wf| RecommendationEntry {
                workflow_id: wf.id.to_string(),
                description: wf.description.to_string(),
                agent: wf.agent.to_string(),
                produces: wf.produces.to_string(),
                prerequisites: wf.prerequisites.iter().map(|s| s.to_string()).collect(),
            }).collect(),
        };

        Ok(format_output(&req.output_format, &json_resp, lines.join("\n")))
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

        // Build JSON response
        let phase_overview = if lower_q.contains("phase") || lower_q.contains("overview") || lower_q.contains("how") {
            Some(Phase::all().iter().map(|p| PhaseOverviewEntry {
                number: p.number(),
                name: p.name().to_string(),
                description: p.description().to_string(),
                workflow_count: idx.get_phase_workflows(*p).len(),
            }).collect())
        } else {
            None
        };

        let track_overview = if lower_q.contains("track") {
            Some(Track::all().iter().map(|t| TrackOverviewEntry {
                name: t.name().to_string(),
                description: t.description().to_string(),
            }).collect())
        } else {
            None
        };

        let json_resp = HelpResponse {
            question: query.clone(),
            context: req.context.clone(),
            results: results.clone(),
            phase_overview,
            track_overview,
        };

        Ok(format_output(&req.output_format, &json_resp, lines.join("\n")))
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

        let json_resp = ReadinessResponse {
            ready: result.ready,
            track: track.name().to_string(),
            project_state: req.project_state.clone(),
            missing_artifacts: result.missing_artifacts.clone(),
            warnings: result.warnings.clone(),
            next_action: result.next_action.clone(),
        };

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

        Ok(format_output(&req.output_format, &json_resp, lines.join("\n")))
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
                    let json_resp = PhaseErrorResponse {
                        error: format!("Unknown phase '{}'", phase_str),
                        valid_phases: vec!["Analysis".to_string(), "Planning".to_string(), "Solutioning".to_string(), "Implementation".to_string()],
                    };
                    let text = format!(
                        "Unknown phase '{}'. Valid phases: Analysis, Planning, Solutioning, Implementation",
                        phase_str
                    );
                    return Ok(format_output(&req.output_format, &json_resp, text));
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

        let json_resp = AgentListResponse {
            phase_filter: req.phase.clone(),
            agents: agents.iter().map(|a| AgentEntry {
                skill_id: a.skill_id.to_string(),
                name: a.name.to_string(),
                persona: a.persona.to_string(),
                workflows: a.primary_workflows.iter().map(|s| s.to_string()).collect(),
            }).collect(),
            total: agents.len(),
        };

        Ok(format_output(&req.output_format, &json_resp, lines.join("\n")))
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
                let json_workflows: Vec<AgentWorkflowEntry> = agent.primary_workflows.iter().map(|wf_id| {
                    if let Some(wf) = idx.get_workflow(wf_id) {
                        AgentWorkflowEntry {
                            id: wf.id.to_string(),
                            description: wf.description.to_string(),
                            phase: wf.phase.name().to_string(),
                            produces: wf.produces.to_string(),
                            tracks: wf.tracks.iter().map(|t| t.name().to_string()).collect(),
                        }
                    } else {
                        AgentWorkflowEntry {
                            id: wf_id.to_string(),
                            description: "(workflow not found in index)".to_string(),
                            phase: String::new(),
                            produces: String::new(),
                            tracks: vec![],
                        }
                    }
                }).collect();

                let json_resp = AgentDetailResponse {
                    skill_id: agent.skill_id.to_string(),
                    name: agent.name.to_string(),
                    persona: agent.persona.to_string(),
                    workflows: json_workflows,
                };

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

                Ok(format_output(&req.output_format, &json_resp, lines.join("\n")))
            }
            None => {
                let all: Vec<&str> = idx.all_agents().iter().map(|a| a.skill_id).collect();
                let json_resp = AgentNotFoundResponse {
                    error: format!("Agent '{}' not found", req.agent_id),
                    available_agents: all.iter().map(|s| s.to_string()).collect(),
                };
                let text = format!(
                    "Agent '{}' not found. Available agents:\n{}",
                    req.agent_id,
                    all.join(", ")
                );
                Ok(format_output(&req.output_format, &json_resp, text))
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
        let SprintGuideResult {
            current_step,
            agent_to_invoke,
            workflow_to_run,
            rationale,
            after_this,
        } = BmadIndex::sprint_guide(&req.sprint_state);

        let json_resp = SprintGuideResponse {
            current_step: current_step.to_string(),
            agent_to_invoke: agent_to_invoke.to_string(),
            workflow_to_run: workflow_to_run.to_string(),
            rationale: rationale.to_string(),
            after_this: after_this.to_string(),
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

        Ok(format_output(&req.output_format, &json_resp, text))
    }

    #[tool(description = "Scan a project directory and detect which BMad Method artifacts exist. \
        Returns a structured report of the project state including BMad installation, \
        planning artifacts (PRD, architecture, epics), implementation artifacts (sprint status), \
        and project context. Also infers the current phase and recommends the next step.")]
    async fn bmad_project_state(
        &self,
        Parameters(req): Parameters<ProjectStateRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let path = std::path::PathBuf::from(&req.project_path);
        if !path.exists() {
            return Err(rmcp::ErrorData::invalid_params(
                format!("Path does not exist: {}", req.project_path),
                None,
            ));
        }
        if !path.is_dir() {
            return Err(rmcp::ErrorData::invalid_params(
                format!("Path is not a directory: {}", req.project_path),
                None,
            ));
        }

        let state = BmadIndex::scan_project_dir(&path).map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to scan directory: {e}"), None)
        })?;

        if !state.bmad_installed {
            let json_resp = ProjectStateResponse {
                project_path: req.project_path.clone(),
                bmad_installed: false,
                prd_found: false,
                architecture_found: false,
                epic_count: 0,
                sprint_status_found: false,
                project_context_found: false,
                suggested_track: String::new(),
                current_phase: String::new(),
                current_phase_number: 0,
                recommended_next_step: String::new(),
            };
            let text = format!(
                "## Project State: {path}\n\n\
                 This does not appear to be a BMad project (`_bmad/` directory not found).\n\n\
                 To get started with the BMad Method, initialize your project with the BMad \
                 configuration directory. Use `bmad_help` for guidance on setting up a new project.",
                path = req.project_path,
            );
            return Ok(format_output(&req.output_format, &json_resp, text));
        }

        let found = |b: bool| if b { "found" } else { "not found" };
        let epic_status = if state.epic_count > 0 {
            format!("{} epic file(s) found", state.epic_count)
        } else {
            "not found".to_string()
        };

        // Build a project_state string for phase inference
        let mut artifacts = Vec::new();
        if state.prd_found {
            artifacts.push("PRD");
        }
        if state.architecture_found {
            artifacts.push("architecture");
        }
        if state.epic_count > 0 {
            artifacts.push("epics");
        }
        if state.sprint_status_found {
            artifacts.push("sprint status");
        }

        let inferred_state = if artifacts.is_empty() {
            "nothing yet".to_string()
        } else {
            format!("has {}", artifacts.join(", has "))
        };

        let completed = BmadIndex::infer_completed_workflows(&inferred_state);
        let phase = BmadIndex::infer_current_phase(&completed);

        let idx = self.index.read().await;
        let recommendations = idx.recommend_next(&completed, None);
        let next_step = if let Some(wf) = recommendations.first() {
            format!("`{}` ({})", wf.id, wf.agent)
        } else {
            "All workflows appear complete.".to_string()
        };

        // Infer track from artifacts
        let suggested_track = if state.prd_found || state.architecture_found || state.epic_count > 0
        {
            "BMad Method"
        } else {
            "Quick Flow (or BMad Method — insufficient artifacts to determine)"
        };

        let json_resp = ProjectStateResponse {
            project_path: req.project_path.clone(),
            bmad_installed: true,
            prd_found: state.prd_found,
            architecture_found: state.architecture_found,
            epic_count: state.epic_count,
            sprint_status_found: state.sprint_status_found,
            project_context_found: state.project_context_found,
            suggested_track: suggested_track.to_string(),
            current_phase: phase.name().to_string(),
            current_phase_number: phase.number(),
            recommended_next_step: next_step.clone(),
        };

        let text = format!(
            "## Project State: {path}\n\n\
             - BMad installed: yes\n\
             - PRD: {prd}\n\
             - Architecture: {arch}\n\
             - Epics: {epics}\n\
             - Sprint status: {sprint}\n\
             - Project context: {ctx}\n\n\
             Suggested track: {track}\n\
             Current phase: {phase_name} (Phase {phase_num})\n\
             Recommended next step: {next_step}",
            path = req.project_path,
            prd = found(state.prd_found),
            arch = found(state.architecture_found),
            epics = epic_status,
            sprint = found(state.sprint_status_found),
            ctx = found(state.project_context_found),
            track = suggested_track,
            phase_name = phase.name(),
            phase_num = phase.number(),
        );

        Ok(format_output(&req.output_format, &json_resp, text))
    }

    #[tool(description = "Refresh the BMad Method documentation cache from the remote source. \
        Requires BMAD_ALLOW_REFRESH=1 and BMAD_DOCS_URL to be set. \
        Fetches the latest docs, validates content, updates the cache, and rebuilds the index. \
        If validation fails, the previous index is preserved.")]
    async fn bmad_refresh_docs(
        &self,
        Parameters(req): Parameters<RefreshDocsRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let allowed = std::env::var("BMAD_ALLOW_REFRESH")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if !allowed {
            let json_resp = RefreshResponse {
                status: "disabled".to_string(),
                message: "Doc refresh is disabled. Set BMAD_ALLOW_REFRESH=1 to enable.".to_string(),
                bytes: None,
            };
            let text = "Doc refresh is disabled. Set BMAD_ALLOW_REFRESH=1 to enable.".to_string();
            return Ok(format_output(&req.output_format, &json_resp, text));
        }

        let url = std::env::var("BMAD_DOCS_URL").map_err(|_| {
            rmcp::ErrorData::invalid_params(
                "BMAD_DOCS_URL is not set. Cannot refresh docs without a source URL.",
                None,
            )
        })?;

        tracing::info!(%url, "refreshing docs from remote source");

        let docs = fetch_and_cache_docs(&url).await.map_err(|e| {
            tracing::error!(%url, error = %e, "doc refresh fetch failed");
            rmcp::ErrorData::internal_error(format!("Failed to fetch docs: {e}"), None)
        })?;

        // Validate before replacing index
        let validation = BmadIndex::validate_docs(&docs);
        if !validation.valid {
            let errors = validation.errors.join("; ");
            tracing::warn!(%url, %errors, "doc validation failed, keeping previous index");
            let json_resp = RefreshResponse {
                status: "validation_failed".to_string(),
                message: format!("Doc refresh failed validation — previous index preserved. Validation errors: {errors}"),
                bytes: None,
            };
            let text = format!(
                "Doc refresh failed validation — previous index preserved.\n\
                 Validation errors: {errors}"
            );
            return Ok(format_output(&req.output_format, &json_resp, text));
        }

        let bytes = docs.len();
        let new_index = BmadIndex::build_with_source(docs, DocSource::Url(url.clone()));
        let mut idx = self.index.write().await;
        *idx = new_index;

        tracing::info!(%url, bytes, "index rebuilt with refreshed docs");

        // Notify all resource subscribers that docs have been updated
        let subs = self.subscriptions.read().await;
        for (uri, peer) in subs.iter() {
            let param = ResourceUpdatedNotificationParam::new(uri.clone());
            if let Err(e) = peer.notify_resource_updated(param).await {
                tracing::warn!(%uri, %e, "failed to notify subscriber of resource update");
            } else {
                tracing::info!(%uri, "notified subscriber of resource update");
            }
        }

        let json_resp = RefreshResponse {
            status: "success".to_string(),
            message: format!("Documentation refreshed successfully ({bytes} bytes). Index rebuilt."),
            bytes: Some(bytes),
        };
        let text = format!(
            "Documentation refreshed successfully ({bytes} bytes). Index rebuilt.",
        );
        Ok(format_output(&req.output_format, &json_resp, text))
    }

    #[tool(description = "Return diagnostic information about the current BMad index state: \
        doc source (embedded, URL, or cache), last refresh time, total workflows parsed, \
        total agents parsed, and doc byte size. No input parameters needed.")]
    async fn bmad_index_status(
        &self,
        Parameters(req): Parameters<IndexStatusRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = self.index.read().await;
        let source = idx.doc_source().to_string();
        let workflows = idx.all_workflow_ids().len();
        let agents = idx.all_agents().len();
        let byte_size = idx.doc_byte_size();
        let uptime = idx
            .last_refresh()
            .map(|t| {
                let elapsed = t.elapsed();
                format!("{:.1}s ago", elapsed.as_secs_f64())
            })
            .unwrap_or_else(|| "unknown".to_string());

        let json_resp = IndexStatusResponse {
            doc_source: source.clone(),
            last_refresh: uptime.clone(),
            total_workflows: workflows,
            total_agents: agents,
            doc_byte_size: byte_size,
        };

        let text = format!(
            "## BMad Index Status\n\n\
             - **Doc source:** {source}\n\
             - **Last refresh:** {uptime}\n\
             - **Total workflows:** {workflows}\n\
             - **Total agents:** {agents}\n\
             - **Doc byte size:** {byte_size}",
        );

        Ok(format_output(&req.output_format, &json_resp, text))
    }

    #[tool(description = "Generate starter BMad Method project files in a target directory. \
        Creates the standard _bmad/ config directory and planning artifact stubs \
        pre-filled with track-appropriate boilerplate and TODO markers. \
        Accepts a track (quick_flow, bmad_method, enterprise) and optional project_dir.")]
    async fn bmad_scaffold(
        &self,
        Parameters(req): Parameters<ScaffoldRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let track = parse_track(&req.track).ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!(
                    "Unknown track '{}'. Valid tracks: quick_flow, bmad_method, enterprise",
                    req.track
                ),
                None,
            )
        })?;

        let project_dir = req
            .project_dir
            .as_deref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));

        if !project_dir.exists() {
            return Err(rmcp::ErrorData::invalid_params(
                format!("Directory does not exist: {}", project_dir.display()),
                None,
            ));
        }
        if !project_dir.is_dir() {
            return Err(rmcp::ErrorData::invalid_params(
                format!("Path is not a directory: {}", project_dir.display()),
                None,
            ));
        }

        let today = {
            use std::time::SystemTime;
            let secs = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let days = secs / 86400;
            // Simple date calculation (good enough for YYYY-MM-DD)
            let (y, m, d) = civil_from_days(days as i64);
            format!("{y:04}-{m:02}-{d:02}")
        };

        let vars = TemplateVars {
            project_name: req.project_name.unwrap_or_default(),
            author: req.author.unwrap_or_default(),
            date: today,
            track: track.name().to_string(),
        };

        let ScaffoldResult {
            files_created,
            track,
            next_steps,
        } = BmadIndex::scaffold_project(&project_dir, track, Some(&vars)).map_err(|e| {
            rmcp::ErrorData::internal_error(
                format!("Failed to scaffold project: {e}"),
                None,
            )
        })?;

        let dir_display = project_dir.display().to_string();

        let json_val = ScaffoldResponse {
            track: track.name().to_string(),
            project_dir: dir_display.clone(),
            files_created: files_created.clone(),
            next_steps: next_steps.iter().map(|s| s.to_string()).collect(),
        };

        let mut lines = vec![format!(
            "## BMad Project Scaffolded\n\n\
             **Track:** {}\n\
             **Directory:** {}\n\n\
             ### Files Created\n",
            track.name(),
            dir_display,
        )];

        for f in &files_created {
            lines.push(format!("- `{f}`"));
        }

        lines.push("\n### Next Steps\n".to_string());
        for step in &next_steps {
            lines.push(format!("- Run `{step}` to fill in the generated stubs"));
        }

        lines.push(format!(
            "\nUse `bmad_project_state` with `project_path: \"{}\"` to verify the scaffolded files.",
            dir_display,
        ));

        if std::env::var("BMAD_TEMPLATES_DIR").is_ok() {
            lines.push("\n> Templates loaded with `BMAD_TEMPLATES_DIR` override active.".to_string());
        }

        let markdown = lines.join("\n");
        Ok(format_output(&req.output_format, &json_val, markdown))
    }
    #[tool(description = "Interactively execute a BMad Method workflow step by step. \
        Supports three actions: 'start' begins a new workflow session, 'next' advances to the \
        next step (optionally recording the result of the current step), and 'status' shows \
        current progress. Session state is persisted in _bmad/sessions/ within the project \
        directory. On start, auto-detects completed steps from existing project artifacts.")]
    async fn bmad_run_workflow(
        &self,
        Parameters(req): Parameters<RunWorkflowRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let project_dir = std::path::PathBuf::from(&req.project_dir);
        if !project_dir.is_dir() {
            return Err(rmcp::ErrorData::invalid_params(
                format!("Not a directory: {}", req.project_dir),
                None,
            ));
        }

        match req.action.as_str() {
            "start" => self.run_workflow_start(&req, &project_dir).await,
            "next" => self.run_workflow_next(&req, &project_dir).await,
            "status" => self.run_workflow_status(&req, &project_dir).await,
            other => Err(rmcp::ErrorData::invalid_params(
                format!("Unknown action '{other}'. Valid actions: start, next, status"),
                None,
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// bmad_run_workflow helper methods
// ---------------------------------------------------------------------------

impl BmadServer {
    async fn run_workflow_start(
        &self,
        req: &RunWorkflowRequest,
        project_dir: &std::path::Path,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let workflow_id = req.workflow_id.as_deref().ok_or_else(|| {
            rmcp::ErrorData::invalid_params("workflow_id is required for action=start", None)
        })?;

        let idx = self.index.read().await;
        let wf = idx.get_workflow(workflow_id).ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!(
                    "Unknown workflow '{workflow_id}'. Use bmad_get_workflow to list available workflows."
                ),
                None,
            )
        })?;
        let wf_desc = wf.description.to_string();
        let wf_agent = wf.agent.to_string();
        let wf_next: Vec<String> = wf.next_steps.iter().map(|s| s.to_string()).collect();
        drop(idx);

        let steps = BmadIndex::get_workflow_steps(workflow_id).ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!("No step definitions available for workflow '{workflow_id}'"),
                None,
            )
        })?;

        let session = BmadIndex::start_session(project_dir, workflow_id).map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to create session: {e}"), None)
        })?;

        BmadIndex::save_session(project_dir, &session).map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to save session: {e}"), None)
        })?;

        if session.completed {
            let next_wf = wf_next.first().cloned();
            let json_resp = RunWorkflowResponse {
                workflow_id: workflow_id.to_string(),
                action: "start".to_string(),
                current_step: session.current_step,
                total_steps: session.total_steps,
                completed: true,
                step: None,
                next_workflow: next_wf.clone(),
                message: "Workflow already completed based on detected project artifacts."
                    .to_string(),
            };
            let text = format!(
                "## Workflow: {wf_id}\n\n\
                 **Status:** Already completed (detected from project artifacts)\n\n\
                 {next}",
                wf_id = workflow_id,
                next = if let Some(ref nw) = next_wf {
                    format!("**Suggested next workflow:** `{nw}`")
                } else {
                    "No further workflows suggested.".to_string()
                },
            );
            return Ok(format_output(&req.output_format, &json_resp, text));
        }

        let step = &steps[session.current_step];
        let step_info = build_step_info(session.current_step, step);
        let json_resp = RunWorkflowResponse {
            workflow_id: workflow_id.to_string(),
            action: "start".to_string(),
            current_step: session.current_step,
            total_steps: session.total_steps,
            completed: false,
            step: Some(step_info),
            next_workflow: None,
            message: format!(
                "Started workflow '{workflow_id}'. Complete the step below, then call with action=next."
            ),
        };
        let text = format!(
            "## Workflow Started: {wf_id}\n\n\
             **Description:** {desc}\n\
             **Agent:** `{agent}`\n\
             **Progress:** Step {cur}/{total}\n\n\
             ---\n\n\
             {step_md}",
            wf_id = workflow_id,
            desc = wf_desc,
            agent = wf_agent,
            cur = session.current_step + 1,
            total = session.total_steps,
            step_md = format_step_markdown(session.current_step, step),
        );
        Ok(format_output(&req.output_format, &json_resp, text))
    }

    async fn run_workflow_next(
        &self,
        req: &RunWorkflowRequest,
        project_dir: &std::path::Path,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let wf_id = if let Some(ref id) = req.workflow_id {
            id.clone()
        } else {
            find_active_session(project_dir).ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    "workflow_id is required (or a session must exist in the project directory)",
                    None,
                )
            })?
        };

        let mut session = BmadIndex::load_session(project_dir, &wf_id)
            .map_err(|e| {
                rmcp::ErrorData::internal_error(format!("Failed to load session: {e}"), None)
            })?
            .ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!("No active session for workflow '{wf_id}'. Use action=start first."),
                    None,
                )
            })?;

        BmadIndex::advance_session(&mut session, req.step_result.clone())
            .map_err(|e| rmcp::ErrorData::invalid_params(e, None))?;

        BmadIndex::save_session(project_dir, &session).map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to save session: {e}"), None)
        })?;

        let steps = BmadIndex::get_workflow_steps(&wf_id);

        if session.completed {
            let idx = self.index.read().await;
            let next_wf = idx
                .get_workflow(&wf_id)
                .and_then(|w| w.next_steps.first().copied())
                .map(String::from);
            drop(idx);

            let json_resp = RunWorkflowResponse {
                workflow_id: wf_id.clone(),
                action: "next".to_string(),
                current_step: session.current_step,
                total_steps: session.total_steps,
                completed: true,
                step: None,
                next_workflow: next_wf.clone(),
                message: "Workflow completed!".to_string(),
            };
            let text = format!(
                "## Workflow Completed: {wf_id}\n\n\
                 All {total} steps finished.\n\n\
                 {next}",
                total = session.total_steps,
                next = if let Some(ref nw) = next_wf {
                    format!(
                        "**Suggested next workflow:** `{nw}`\n\
                         Use `bmad_run_workflow` with `action: \"start\"` and `workflow_id: \"{nw}\"` to continue."
                    )
                } else {
                    "No further workflows suggested.".to_string()
                },
            );
            return Ok(format_output(&req.output_format, &json_resp, text));
        }

        let step = steps.and_then(|s| s.get(session.current_step));

        let (step_info, step_md) = if let Some(step) = step {
            (
                Some(build_step_info(session.current_step, step)),
                format_step_markdown(session.current_step, step),
            )
        } else {
            (None, "Step data not available.".to_string())
        };

        let json_resp = RunWorkflowResponse {
            workflow_id: wf_id.clone(),
            action: "next".to_string(),
            current_step: session.current_step,
            total_steps: session.total_steps,
            completed: false,
            step: step_info,
            next_workflow: None,
            message: format!(
                "Advanced to step {}/{}.",
                session.current_step + 1,
                session.total_steps
            ),
        };
        let text = format!(
            "## Workflow: {wf_id}\n\n\
             **Progress:** Step {cur}/{total}\n\n\
             ---\n\n\
             {step_md}",
            cur = session.current_step + 1,
            total = session.total_steps,
        );
        Ok(format_output(&req.output_format, &json_resp, text))
    }

    async fn run_workflow_status(
        &self,
        req: &RunWorkflowRequest,
        project_dir: &std::path::Path,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let wf_id = req
            .workflow_id
            .as_deref()
            .map(String::from)
            .or_else(|| find_active_session(project_dir));

        let wf_id = wf_id.ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                "workflow_id is required (or a session must exist in the project directory)",
                None,
            )
        })?;

        let session = BmadIndex::load_session(project_dir, &wf_id)
            .map_err(|e| {
                rmcp::ErrorData::internal_error(format!("Failed to load session: {e}"), None)
            })?
            .ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!("No session found for workflow '{wf_id}'. Use action=start first."),
                    None,
                )
            })?;

        let steps = BmadIndex::get_workflow_steps(&wf_id);
        let step = if !session.completed {
            steps.and_then(|s| s.get(session.current_step))
        } else {
            None
        };

        let step_info = step.map(|s| build_step_info(session.current_step, s));

        let json_resp = RunWorkflowResponse {
            workflow_id: wf_id.clone(),
            action: "status".to_string(),
            current_step: session.current_step,
            total_steps: session.total_steps,
            completed: session.completed,
            step: step_info,
            next_workflow: None,
            message: if session.completed {
                "Workflow completed.".to_string()
            } else {
                format!(
                    "In progress: step {}/{}.",
                    session.current_step + 1,
                    session.total_steps
                )
            },
        };

        let status_label = if session.completed {
            "Completed"
        } else {
            "In Progress"
        };

        let mut text = format!(
            "## Workflow Status: {wf_id}\n\n\
             **Status:** {status_label}\n\
             **Progress:** {cur}/{total} steps\n\
             **Started:** {started}\n\
             **Last updated:** {updated}\n",
            cur = session.current_step,
            total = session.total_steps,
            started = session.started_at,
            updated = session.updated_at,
        );

        if !session.completed
            && let Some(s) = step
        {
            text.push_str(&format!(
                "\n---\n\n### Current Step\n\n{}",
                format_step_markdown(session.current_step, s),
            ));
        }

        Ok(format_output(&req.output_format, &json_resp, text))
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers for workflow steps
// ---------------------------------------------------------------------------

fn build_step_info(index: usize, step: &WorkflowStep) -> RunWorkflowStepInfo {
    RunWorkflowStepInfo {
        number: index + 1,
        title: step.title.to_string(),
        instructions: step.instructions.to_string(),
        expected_output: step.expected_output.to_string(),
        agent_guidance: step.agent_guidance.to_string(),
    }
}

fn format_step_markdown(index: usize, step: &WorkflowStep) -> String {
    format!(
        "### Step {num}: {title}\n\n\
         **Instructions:** {instructions}\n\n\
         **Expected output:** {expected_output}\n\n\
         **Agent guidance:** {agent_guidance}",
        num = index + 1,
        title = step.title,
        instructions = step.instructions,
        expected_output = step.expected_output,
        agent_guidance = step.agent_guidance,
    )
}

/// Find the most recently modified session file in the project's _bmad/sessions/ directory.
fn find_active_session(project_dir: &std::path::Path) -> Option<String> {
    let sessions_dir = project_dir.join("_bmad").join("sessions");
    if !sessions_dir.is_dir() {
        return None;
    }

    std::fs::read_dir(&sessions_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        .and_then(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
        })
}


/// Convert days since Unix epoch to (year, month, day).
fn civil_from_days(mut z: i64) -> (i64, u32, u32) {
    z += 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[tool_handler]
impl ServerHandler for BmadServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_resources_subscribe()
                .build(),
        )
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
             and workflows. Resources are available at bmad:// URIs for browsing \
             documentation on phases, workflows, agents, and tracks."
                .to_string(),
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        let idx = self.index.read().await;
        let mut resources = Vec::new();

        // Full docs resource
        resources.push(
            RawResource::new("bmad://docs", "BMad Method Documentation")
                .with_description("Complete BMad Method documentation")
                .with_mime_type("text/plain")
                .no_annotation(),
        );

        // Phase resources
        for phase in Phase::all() {
            let uri = format!("bmad://phases/{}", phase.name().to_lowercase());
            resources.push(
                RawResource::new(&uri, format!("Phase {}: {}", phase.number(), phase.name()))
                    .with_description(phase.description())
                    .with_mime_type("text/markdown")
                    .no_annotation(),
            );
        }

        // Track resources
        for track in Track::all() {
            let uri = format!(
                "bmad://tracks/{}",
                track.name().to_lowercase().replace(' ', "-")
            );
            resources.push(
                RawResource::new(&uri, format!("{} Track", track.name()))
                    .with_description(track.description())
                    .with_mime_type("text/markdown")
                    .no_annotation(),
            );
        }

        // Workflow resources
        for wf_id in idx.all_workflow_ids() {
            let uri = format!("bmad://workflows/{wf_id}");
            let desc = idx
                .get_workflow(wf_id)
                .map(|w| w.description)
                .unwrap_or("BMad workflow");
            resources.push(
                RawResource::new(&uri, wf_id)
                    .with_description(desc)
                    .with_mime_type("text/markdown")
                    .no_annotation(),
            );
        }

        // Agent resources
        for agent in idx.all_agents() {
            let uri = format!("bmad://agents/{}", agent.skill_id);
            resources.push(
                RawResource::new(&uri, agent.name)
                    .with_description(format!("{} — {}", agent.persona, agent.skill_id))
                    .with_mime_type("text/markdown")
                    .no_annotation(),
            );
        }

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, rmcp::ErrorData> {
        let templates = vec![
            RawResourceTemplate::new("bmad://phases/{phase}", "Phase by name")
                .with_description(
                    "BMad Method phase details. Use: analysis, planning, solutioning, implementation",
                )
                .with_mime_type("text/markdown")
                .no_annotation(),
            RawResourceTemplate::new("bmad://workflows/{workflow_id}", "Workflow by ID")
                .with_description("BMad Method workflow details by skill ID")
                .with_mime_type("text/markdown")
                .no_annotation(),
            RawResourceTemplate::new("bmad://agents/{agent_id}", "Agent by skill ID")
                .with_description("BMad Method agent details by skill ID")
                .with_mime_type("text/markdown")
                .no_annotation(),
            RawResourceTemplate::new("bmad://tracks/{track}", "Track by name")
                .with_description(
                    "BMad Method planning track details. Use: quick-flow, bmad-method, enterprise",
                )
                .with_mime_type("text/markdown")
                .no_annotation(),
        ];

        Ok(ListResourceTemplatesResult {
            resource_templates: templates,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        let uri = &request.uri;
        let idx = self.index.read().await;

        let content = if uri == "bmad://docs" {
            idx.raw_docs().to_string()
        } else if let Some(phase_name) = uri.strip_prefix("bmad://phases/") {
            let phase = parse_phase(phase_name).ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!("Unknown phase '{phase_name}'. Valid: analysis, planning, solutioning, implementation"),
                    None,
                )
            })?;
            let workflows = idx.get_phase_workflows(phase);
            let wf_details: Vec<String> = workflows
                .iter()
                .filter_map(|id| {
                    idx.get_workflow(id).map(|wf| {
                        format!(
                            "- **`{}`** — {}\n  Agent: `{}` | Produces: {}",
                            wf.id, wf.description, wf.agent, wf.produces
                        )
                    })
                })
                .collect();
            format!(
                "# Phase {}: {}\n\n{}\n\n## Workflows\n\n{}",
                phase.number(),
                phase.name(),
                phase.description(),
                if wf_details.is_empty() {
                    "No workflows in this phase.".to_string()
                } else {
                    wf_details.join("\n")
                }
            )
        } else if let Some(wf_id) = uri.strip_prefix("bmad://workflows/") {
            let wf = idx.get_workflow(wf_id).ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!("Workflow '{wf_id}' not found"),
                    None,
                )
            })?;
            let tracks: Vec<&str> = wf.tracks.iter().map(|t| t.name()).collect();
            format!(
                "# {id}\n\n\
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
            )
        } else if let Some(agent_id) = uri.strip_prefix("bmad://agents/") {
            let agent = idx.get_agent(agent_id).ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!("Agent '{agent_id}' not found"),
                    None,
                )
            })?;
            let mut lines = vec![format!(
                "# {name}\n\n\
                 **Skill ID:** `{skill_id}`\n\
                 **Persona:** {persona}\n\n\
                 ## Primary Workflows\n",
                name = agent.name,
                skill_id = agent.skill_id,
                persona = agent.persona,
            )];
            for wf_id in &agent.primary_workflows {
                if let Some(wf) = idx.get_workflow(wf_id) {
                    let tracks: Vec<&str> = wf.tracks.iter().map(|t| t.name()).collect();
                    lines.push(format!(
                        "- **`{}`** — {}\n  Phase: {} | Produces: {} | Tracks: {}",
                        wf.id,
                        wf.description,
                        wf.phase.name(),
                        wf.produces,
                        tracks.join(", "),
                    ));
                } else {
                    lines.push(format!("- `{wf_id}` — (not found in index)"));
                }
            }
            lines.join("\n")
        } else if let Some(track_name) = uri.strip_prefix("bmad://tracks/") {
            let normalized = track_name.replace('-', " ");
            let track = parse_track(&normalized).ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!("Unknown track '{track_name}'. Valid: quick-flow, bmad-method, enterprise"),
                    None,
                )
            })?;
            let wfs = idx.get_track_workflows(track);
            let mut lines = vec![format!(
                "# {} Track\n\n{}\n\n## Phases\n\n{}\n\n## Workflows\n",
                track.name(),
                track.description(),
                track
                    .phases()
                    .iter()
                    .map(|p| format!("- Phase {}: {}", p.number(), p.name()))
                    .collect::<Vec<_>>()
                    .join("\n"),
            )];
            for wf in &wfs {
                lines.push(format!(
                    "- **`{}`** — {} (Phase {}: {})",
                    wf.id,
                    wf.description,
                    wf.phase.number(),
                    wf.phase.name(),
                ));
            }
            lines.join("\n")
        } else {
            return Err(rmcp::ErrorData::invalid_params(
                format!("Unknown resource URI: {uri}"),
                None,
            ));
        };

        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(content, uri.clone()).with_mime_type("text/markdown"),
        ]))
    }

    async fn subscribe(
        &self,
        request: SubscribeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
        let uri = request.uri;
        tracing::info!(%uri, "client subscribed to resource");
        self.subscriptions
            .write()
            .await
            .push((uri, context.peer.clone()));
        Ok(())
    }

    async fn unsubscribe(
        &self,
        request: UnsubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
        let uri = &request.uri;
        tracing::info!(%uri, "client unsubscribed from resource");
        self.subscriptions.write().await.retain(|(u, _)| u != uri);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

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

    let (docs, source) = load_startup_docs().await;
    let index = Arc::new(RwLock::new(BmadIndex::build_with_source(docs, source)));
    tracing::info!("index built (lazy singleton, reused across all tool calls)");

    let transport = std::env::var("BMAD_TRANSPORT")
        .unwrap_or_else(|_| "stdio".to_string());

    match transport.as_str() {
        "sse" | "http" => {
            let host = std::env::var("BMAD_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let port: u16 = std::env::var("BMAD_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("BMAD_PORT must be a valid port number");

            let addr = format!("{host}:{port}");
            tracing::info!(%addr, "starting SSE/HTTP transport");

            let ct = tokio_util::sync::CancellationToken::new();

            let config = StreamableHttpServerConfig {
                cancellation_token: ct.child_token(),
                ..Default::default()
            };

            let service = StreamableHttpService::new(
                move || Ok(BmadServer::new(index.clone())),
                LocalSessionManager::default().into(),
                config,
            );

            let router = axum::Router::new().nest_service("/mcp", service);

            let listener = tokio::net::TcpListener::bind(&addr).await?;
            tracing::info!(%addr, "listening for MCP SSE connections");

            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    tokio::signal::ctrl_c().await.ok();
                    tracing::info!("shutdown signal received");
                    ct.cancel();
                })
                .await?;
        }
        _ => {
            let server = BmadServer::new(index);
            let service = server.serve(stdio()).await?;
            service.waiting().await?;
        }
    }

    Ok(())
}
