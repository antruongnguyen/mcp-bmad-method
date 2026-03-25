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
                 for each phase, and find the right planning track for your project. The server \
                 knows about all phases (Analysis, Planning, Solutioning, Implementation), all \
                 three planning tracks (Quick Flow, BMad Method, Enterprise), and all default \
                 agents and their workflows."
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
