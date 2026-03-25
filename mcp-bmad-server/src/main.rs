mod bmad_index;

use bmad_index::{BmadIndex, Phase, Track};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

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

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct BmadServer {
    tool_router: ToolRouter<Self>,
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

#[tool_router]
impl BmadServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get metadata for a BMad Method workflow by its skill id. \
        Returns the workflow description, phase, agent, outputs, prerequisites, \
        next steps, and applicable planning tracks.")]
    async fn bmad_get_workflow(
        &self,
        Parameters(req): Parameters<GetWorkflowRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = BmadIndex::build();
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
        let idx = BmadIndex::build();
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
        let idx = BmadIndex::build();
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
        let idx = BmadIndex::build();
        let completed = BmadIndex::infer_completed_workflows(&req.project_state);
        let phase = BmadIndex::infer_current_phase(&completed);

        let recommendations = idx.recommend_next(
            &completed,
            req.last_workflow.as_deref(),
        );

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
        tracks, and core tools. Searches the embedded BMad documentation and structured index \
        to provide contextual answers.")]
    async fn bmad_help(
        &self,
        Parameters(req): Parameters<HelpRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let idx = BmadIndex::build();
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
            let docs = BmadIndex::raw_docs();
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

    let server = BmadServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
