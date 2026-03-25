use std::sync::Arc;

use rmcp::ServiceExt;
use rmcp::handler::server::wrapper::Parameters;
use tokio::sync::RwLock;

use crate::bmad_index::BmadIndex;
use crate::{
    AgentInfoRequest, BmadServer, CheckReadinessRequest, GetNextStepsRequest,
    GetTrackWorkflowsRequest, GetWorkflowRequest, HelpRequest, IndexStatusRequest,
    ListAgentsRequest, NextStepRequest, ProjectStateRequest, RunWorkflowRequest, ScaffoldRequest, SprintGuideRequest,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `BmadServer` backed by the embedded docs for testing.
fn server() -> BmadServer {
    let index = Arc::new(RwLock::new(BmadIndex::build()));
    BmadServer::new(index)
}

/// Extract the text body from a `CallToolResult`.
fn text_of(result: Result<rmcp::model::CallToolResult, rmcp::ErrorData>) -> String {
    let call_result = result.expect("tool handler returned an error");
    call_result
        .content
        .into_iter()
        .filter_map(|c| match c.raw {
            rmcp::model::RawContent::Text(t) => Some(t.text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// =========================================================================
// bmad_get_workflow
// =========================================================================

#[tokio::test]
async fn get_workflow_valid_id() {
    let srv = server();
    let result = srv
        .bmad_get_workflow(Parameters(GetWorkflowRequest {
            workflow_id: "bmad-create-prd".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("bmad-create-prd"), "should echo the workflow id");
    assert!(text.contains("PRD.md"), "should mention produces artifact");
    assert!(text.contains("bmad-pm"), "should mention the agent");
    assert!(text.contains("Planning"), "should mention the phase");
}

#[tokio::test]
async fn get_workflow_unknown_id() {
    let srv = server();
    let result = srv
        .bmad_get_workflow(Parameters(GetWorkflowRequest {
            workflow_id: "nonexistent-workflow".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("not found"), "should indicate workflow not found");
    assert!(
        text.contains("bmad-create-prd"),
        "should list available workflows"
    );
}

// =========================================================================
// bmad_get_next_steps
// =========================================================================

#[tokio::test]
async fn get_next_steps_analysis() {
    let srv = server();
    let result = srv
        .bmad_get_next_steps(Parameters(GetNextStepsRequest {
            phase: "Analysis".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("bmad-create-prd"), "should suggest PRD after analysis");
    assert!(text.contains("Phase 1"), "should reference phase number");
}

#[tokio::test]
async fn get_next_steps_planning() {
    let srv = server();
    let result = srv
        .bmad_get_next_steps(Parameters(GetNextStepsRequest {
            phase: "Planning".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("bmad-create-architecture"));
}

#[tokio::test]
async fn get_next_steps_solutioning() {
    let srv = server();
    let result = srv
        .bmad_get_next_steps(Parameters(GetNextStepsRequest {
            phase: "Solutioning".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("bmad-sprint-planning"));
}

#[tokio::test]
async fn get_next_steps_implementation() {
    let srv = server();
    let result = srv
        .bmad_get_next_steps(Parameters(GetNextStepsRequest {
            phase: "Implementation".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("bmad-retrospective"));
}

#[tokio::test]
async fn get_next_steps_unknown_phase() {
    let srv = server();
    let result = srv
        .bmad_get_next_steps(Parameters(GetNextStepsRequest {
            phase: "Nonexistent".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(
        text.contains("Unknown phase"),
        "should report unknown phase"
    );
    assert!(
        text.contains("Analysis"),
        "should list valid phases"
    );
}

// =========================================================================
// bmad_get_track_workflows
// =========================================================================

#[tokio::test]
async fn get_track_workflows_quick_flow() {
    let srv = server();
    let result = srv
        .bmad_get_track_workflows(Parameters(GetTrackWorkflowsRequest {
            track: "Quick Flow".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Quick Flow Track"), "should have track header");
    assert!(text.contains("bmad-quick-dev"), "should include quick-dev workflow");
}

#[tokio::test]
async fn get_track_workflows_bmad_method() {
    let srv = server();
    let result = srv
        .bmad_get_track_workflows(Parameters(GetTrackWorkflowsRequest {
            track: "BMad Method".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("BMad Method Track"));
    assert!(text.contains("bmad-create-prd"));
    assert!(text.contains("bmad-create-architecture"));
}

#[tokio::test]
async fn get_track_workflows_enterprise() {
    let srv = server();
    let result = srv
        .bmad_get_track_workflows(Parameters(GetTrackWorkflowsRequest {
            track: "Enterprise".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Enterprise Track"));
    assert!(text.contains("bmad-create-prd"));
}

#[tokio::test]
async fn get_track_workflows_unknown() {
    let srv = server();
    let result = srv
        .bmad_get_track_workflows(Parameters(GetTrackWorkflowsRequest {
            track: "FooTrack".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Unknown track"));
    assert!(text.contains("Quick Flow"));
}

// =========================================================================
// bmad_next_step
// =========================================================================

#[tokio::test]
async fn next_step_has_prd_only() {
    let srv = server();
    let result = srv
        .bmad_next_step(Parameters(NextStepRequest {
            project_state: "has PRD".to_string(),
            last_workflow: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Next Step"), "should contain header");
    assert!(
        text.contains("bmad-create-architecture") || text.contains("bmad-create-ux-design"),
        "should recommend architecture or UX after PRD"
    );
}

#[tokio::test]
async fn next_step_has_prd_and_architecture() {
    let srv = server();
    let result = srv
        .bmad_next_step(Parameters(NextStepRequest {
            project_state: "has PRD, has architecture".to_string(),
            last_workflow: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(
        text.contains("bmad-sprint-planning") || text.contains("bmad-create-epics-and-stories"),
        "should recommend epics or sprint planning after PRD + architecture"
    );
}

#[tokio::test]
async fn next_step_has_all_artifacts() {
    let srv = server();
    let result = srv
        .bmad_next_step(Parameters(NextStepRequest {
            project_state: "has PRD, has architecture, has epics, sprint status done".to_string(),
            last_workflow: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    // In implementation phase, should recommend retrospective or story creation
    assert!(
        text.contains("Implementation") || text.contains("bmad-retrospective") || text.contains("bmad-create-story"),
        "should be in implementation phase: {text}"
    );
}

#[tokio::test]
async fn next_step_with_last_workflow() {
    let srv = server();
    let result = srv
        .bmad_next_step(Parameters(NextStepRequest {
            project_state: "has PRD".to_string(),
            last_workflow: Some("bmad-create-prd".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(
        text.contains("bmad-create-architecture") || text.contains("bmad-create-ux-design"),
        "should follow PRD's next_steps"
    );
}

#[tokio::test]
async fn next_step_empty_state() {
    let srv = server();
    let result = srv
        .bmad_next_step(Parameters(NextStepRequest {
            project_state: "nothing yet".to_string(),
            last_workflow: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(
        text.contains("Analysis") || text.contains("bmad-create-prd"),
        "empty state should start at analysis"
    );
}

// =========================================================================
// bmad_help
// =========================================================================

#[tokio::test]
async fn help_pm_agent() {
    let srv = server();
    let result = srv
        .bmad_help(Parameters(HelpRequest {
            question: "bmad-pm".to_string(),
            context: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(
        text.contains("Product Manager") || text.contains("bmad-pm"),
        "should find PM agent info"
    );
}

#[tokio::test]
async fn help_quick_flow() {
    let srv = server();
    let result = srv
        .bmad_help(Parameters(HelpRequest {
            question: "what is Quick Flow?".to_string(),
            context: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(
        text.contains("Quick Flow"),
        "should find Quick Flow info"
    );
}

#[tokio::test]
async fn help_with_context() {
    let srv = server();
    let result = srv
        .bmad_help(Parameters(HelpRequest {
            question: "what should I do next?".to_string(),
            context: Some("BMad Method track, Planning phase".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("BMad Method"), "should echo context");
}

#[tokio::test]
async fn help_no_results_gives_suggestions() {
    let srv = server();
    let result = srv
        .bmad_help(Parameters(HelpRequest {
            question: "xyzzyfoobarbaz999".to_string(),
            context: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    // Should either have no results message or at least not crash
    assert!(
        text.contains("No specific results") || text.contains("Results") || text.contains("BMad"),
        "should handle no-match gracefully"
    );
}

// =========================================================================
// bmad_check_readiness
// =========================================================================

#[tokio::test]
async fn check_readiness_bmad_ready() {
    let srv = server();
    let result = srv
        .bmad_check_readiness(Parameters(CheckReadinessRequest {
            project_state: "PRD done, architecture done, epics created".to_string(),
            track: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("READY"), "should be ready");
    assert!(!text.contains("NOT READY"), "should not be NOT READY");
    assert!(text.contains("bmad-sprint-planning"), "should suggest sprint planning");
}

#[tokio::test]
async fn check_readiness_bmad_missing_prd() {
    let srv = server();
    let result = srv
        .bmad_check_readiness(Parameters(CheckReadinessRequest {
            project_state: "architecture done, epics created".to_string(),
            track: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("NOT READY"), "should not be ready");
    assert!(text.contains("PRD"), "should mention missing PRD");
    assert!(text.contains("bmad-create-prd"), "should suggest creating PRD");
}

#[tokio::test]
async fn check_readiness_bmad_missing_architecture() {
    let srv = server();
    let result = srv
        .bmad_check_readiness(Parameters(CheckReadinessRequest {
            project_state: "PRD done, epics created".to_string(),
            track: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("NOT READY"));
    assert!(text.contains("architecture"));
}

#[tokio::test]
async fn check_readiness_quick_flow_ready() {
    let srv = server();
    let result = srv
        .bmad_check_readiness(Parameters(CheckReadinessRequest {
            project_state: "tech-spec done".to_string(),
            track: Some("Quick Flow".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("READY"));
    assert!(!text.contains("NOT READY"));
    assert!(text.contains("Quick Flow"), "should mention the track");
}

#[tokio::test]
async fn check_readiness_quick_flow_not_ready() {
    let srv = server();
    let result = srv
        .bmad_check_readiness(Parameters(CheckReadinessRequest {
            project_state: "nothing yet".to_string(),
            track: Some("Quick Flow".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("NOT READY"));
    assert!(text.contains("tech-spec"));
}

#[tokio::test]
async fn check_readiness_enterprise_missing_security() {
    let srv = server();
    let result = srv
        .bmad_check_readiness(Parameters(CheckReadinessRequest {
            project_state: "PRD done, architecture done, DevOps done, epics created".to_string(),
            track: Some("Enterprise".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("NOT READY"));
    assert!(text.contains("security"), "should mention missing security docs");
}

#[tokio::test]
async fn check_readiness_default_track_is_bmad() {
    let srv = server();
    let result = srv
        .bmad_check_readiness(Parameters(CheckReadinessRequest {
            project_state: "PRD done, architecture done, epics created".to_string(),
            track: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("BMad Method"), "default track should be BMad Method");
}

// =========================================================================
// bmad_list_agents
// =========================================================================

#[tokio::test]
async fn list_agents_no_filter() {
    let srv = server();
    let result = srv
        .bmad_list_agents(Parameters(ListAgentsRequest { phase: None, output_format: None }))
        .await;
    let text = text_of(result);
    assert!(text.contains("BMad Agents"), "should have agents header");
    assert!(text.contains("bmad-dev"), "should list developer");
    assert!(text.contains("bmad-pm"), "should list PM");
    assert!(text.contains("bmad-architect"), "should list architect");
    assert!(text.contains("9 agent(s)"), "should show 9 agents total");
}

#[tokio::test]
async fn list_agents_filter_by_phase() {
    let srv = server();
    let result = srv
        .bmad_list_agents(Parameters(ListAgentsRequest {
            phase: Some("Implementation".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("bmad-dev"), "developer should be in implementation phase");
    assert!(text.contains("bmad-sm"), "SM should be in implementation phase");
    // Analyst should NOT be listed for implementation
    assert!(!text.contains("bmad-analyst"), "analyst should not be in implementation");
}

#[tokio::test]
async fn list_agents_filter_analysis() {
    let srv = server();
    let result = srv
        .bmad_list_agents(Parameters(ListAgentsRequest {
            phase: Some("Analysis".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("bmad-analyst"), "analyst should be in analysis phase");
    assert!(!text.contains("bmad-dev"), "developer should not be in analysis");
}

#[tokio::test]
async fn list_agents_unknown_phase() {
    let srv = server();
    let result = srv
        .bmad_list_agents(Parameters(ListAgentsRequest {
            phase: Some("Nonexistent".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Unknown phase"));
}

// =========================================================================
// bmad_agent_info
// =========================================================================

#[tokio::test]
async fn agent_info_valid() {
    let srv = server();
    let result = srv
        .bmad_agent_info(Parameters(AgentInfoRequest {
            agent_id: "bmad-dev".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Developer"), "should include agent name");
    assert!(text.contains("Amelia"), "should include persona");
    assert!(text.contains("bmad-dev-story"), "should list primary workflows");
    assert!(text.contains("bmad-code-review"), "should list code-review workflow");
    assert!(text.contains("How to invoke"), "should include invocation instructions");
}

#[tokio::test]
async fn agent_info_architect() {
    let srv = server();
    let result = srv
        .bmad_agent_info(Parameters(AgentInfoRequest {
            agent_id: "bmad-architect".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Architect"));
    assert!(text.contains("Winston"));
    assert!(text.contains("bmad-create-architecture"));
}

#[tokio::test]
async fn agent_info_unknown() {
    let srv = server();
    let result = srv
        .bmad_agent_info(Parameters(AgentInfoRequest {
            agent_id: "nonexistent".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("not found"), "should indicate agent not found");
    assert!(text.contains("bmad-dev"), "should list available agents");
}

// =========================================================================
// bmad_sprint_guide
// =========================================================================

#[tokio::test]
async fn sprint_guide_story_created() {
    let srv = server();
    let result = srv
        .bmad_sprint_guide(Parameters(SprintGuideRequest {
            sprint_state: "story file created but not yet implemented".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Story implementation"), "should detect story implementation step");
    assert!(text.contains("bmad-agent-dev"), "should invoke developer agent");
    assert!(text.contains("bmad-dev-story"), "should run dev-story workflow");
    assert!(text.contains("Build cycle reference"), "should include cycle reference");
}

#[tokio::test]
async fn sprint_guide_story_implemented() {
    let srv = server();
    let result = srv
        .bmad_sprint_guide(Parameters(SprintGuideRequest {
            sprint_state: "story implemented, waiting for review".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Code review"));
    assert!(text.contains("bmad-code-review"));
}

#[tokio::test]
async fn sprint_guide_epic_complete() {
    let srv = server();
    let result = srv
        .bmad_sprint_guide(Parameters(SprintGuideRequest {
            sprint_state: "all stories in epic done".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Epic retrospective"));
    assert!(text.contains("bmad-retrospective"));
    assert!(text.contains("bmad-agent-sm"));
}

#[tokio::test]
async fn sprint_guide_no_sprint() {
    let srv = server();
    let result = srv
        .bmad_sprint_guide(Parameters(SprintGuideRequest {
            sprint_state: "brand new project, no sprint plan".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Sprint initialization"));
    assert!(text.contains("bmad-sprint-planning"));
}

#[tokio::test]
async fn sprint_guide_fallback() {
    let srv = server();
    let result = srv
        .bmad_sprint_guide(Parameters(SprintGuideRequest {
            sprint_state: "random unrelated text".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Sprint initialization"), "unknown state should fallback to sprint init");
    assert!(text.contains("bmad-sprint-planning"));
}

// =========================================================================
// bmad_project_state
// =========================================================================

/// Create a temp directory with full BMad output structure for tool tests.
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
        dir.join("_bmad-output/implementation-artifacts/sprint-status.yaml"),
        "status: active",
    )
    .unwrap();
    fs::write(dir.join("_bmad-output/project-context.md"), "# Context").unwrap();
}

#[tokio::test]
async fn project_state_full_project() {
    let tmp = tempfile::tempdir().unwrap();
    make_bmad_project(tmp.path());

    let srv = server();
    let result = srv
        .bmad_project_state(Parameters(ProjectStateRequest {
            project_path: tmp.path().to_string_lossy().to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("BMad installed: yes"));
    assert!(text.contains("PRD: found"));
    assert!(text.contains("Architecture: found"));
    assert!(text.contains("2 epic file(s) found"));
    assert!(text.contains("Sprint status: found"));
    assert!(text.contains("Project context: found"));
    assert!(text.contains("Suggested track:"));
    assert!(text.contains("Current phase:"));
    assert!(text.contains("Recommended next step:"));
}

#[tokio::test]
async fn project_state_not_bmad_project() {
    let tmp = tempfile::tempdir().unwrap();
    // Empty directory

    let srv = server();
    let result = srv
        .bmad_project_state(Parameters(ProjectStateRequest {
            project_path: tmp.path().to_string_lossy().to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(
        text.contains("does not appear to be a BMad project"),
        "should indicate not a BMad project"
    );
    assert!(text.contains("_bmad/"));
}

#[tokio::test]
async fn project_state_nonexistent_path() {
    let srv = server();
    let result = srv
        .bmad_project_state(Parameters(ProjectStateRequest {
            project_path: "/nonexistent/path/xyzzy".to_string(),
            output_format: None,
        }))
        .await;
    assert!(result.is_err(), "should return error for nonexistent path");
}

#[tokio::test]
async fn project_state_partial_artifacts() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("_bmad")).unwrap();
    std::fs::create_dir_all(tmp.path().join("_bmad-output/planning-artifacts")).unwrap();
    std::fs::write(
        tmp.path().join("_bmad-output/planning-artifacts/PRD.md"),
        "# PRD",
    )
    .unwrap();

    let srv = server();
    let result = srv
        .bmad_project_state(Parameters(ProjectStateRequest {
            project_path: tmp.path().to_string_lossy().to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("PRD: found"));
    assert!(text.contains("Architecture: not found"));
    assert!(text.contains("Sprint status: not found"));
}

// =========================================================================
// bmad_index_status
// =========================================================================

#[tokio::test]
async fn index_status_returns_diagnostics() {
    let srv = server();
    let result = srv
        .bmad_index_status(Parameters(IndexStatusRequest { output_format: None }))
        .await;
    let text = text_of(result);
    assert!(text.contains("BMad Index Status"), "should have status header");
    assert!(text.contains("Doc source:"), "should show doc source");
    assert!(text.contains("embedded"), "default source should be embedded");
    assert!(text.contains("Total workflows:"), "should show workflow count");
    assert!(text.contains("Total agents:"), "should show agent count");
    assert!(text.contains("Doc byte size:"), "should show byte size");
    assert!(text.contains("Last refresh:"), "should show refresh time");
}

#[tokio::test]
async fn index_status_shows_correct_counts() {
    let srv = server();
    let result = srv
        .bmad_index_status(Parameters(IndexStatusRequest { output_format: None }))
        .await;
    let text = text_of(result);
    // Should have at least 18 workflows and 9 agents
    assert!(text.contains("18") || text.contains("19") || text.contains("20"),
        "should show workflow count around 18+: {text}");
    assert!(text.contains("9"), "should show 9 agents");
}

// =========================================================================
// bmad_scaffold
// =========================================================================

#[tokio::test]
async fn scaffold_quick_flow() {
    let tmp = tempfile::tempdir().unwrap();
    let srv = server();
    let result = srv
        .bmad_scaffold(Parameters(ScaffoldRequest {
            track: "quick_flow".to_string(),
            project_dir: Some(tmp.path().to_string_lossy().to_string()),
            project_name: None,
            author: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("BMad Project Scaffolded"), "should have scaffold header");
    assert!(text.contains("Quick Flow"), "should mention the track");
    assert!(text.contains("tech-spec.md"), "should list tech-spec file");
    assert!(text.contains("project-context.md"), "should list project context file");
    assert!(text.contains("bmad-quick-dev"), "should suggest bmad-quick-dev as next step");

    // Verify files actually exist
    assert!(tmp.path().join("_bmad").is_dir());
    assert!(tmp.path().join("_bmad-output/planning-artifacts/tech-spec.md").is_file());
    assert!(tmp.path().join("_bmad-output/project-context.md").is_file());
}

#[tokio::test]
async fn scaffold_bmad_method() {
    let tmp = tempfile::tempdir().unwrap();
    let srv = server();
    let result = srv
        .bmad_scaffold(Parameters(ScaffoldRequest {
            track: "bmad_method".to_string(),
            project_dir: Some(tmp.path().to_string_lossy().to_string()),
            project_name: None,
            author: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("BMad Method"), "should mention BMad Method track");
    assert!(text.contains("PRD.md"), "should list PRD");
    assert!(text.contains("architecture.md"), "should list architecture");
    assert!(text.contains("epic-1.md"), "should list epic");
    assert!(text.contains("bmad-create-prd"), "should suggest create-prd");

    // Verify files exist
    assert!(tmp.path().join("_bmad-output/planning-artifacts/PRD.md").is_file());
    assert!(tmp.path().join("_bmad-output/planning-artifacts/architecture.md").is_file());
    assert!(tmp.path().join("_bmad-output/planning-artifacts/epics/epic-1.md").is_file());
}

#[tokio::test]
async fn scaffold_enterprise() {
    let tmp = tempfile::tempdir().unwrap();
    let srv = server();
    let result = srv
        .bmad_scaffold(Parameters(ScaffoldRequest {
            track: "enterprise".to_string(),
            project_dir: Some(tmp.path().to_string_lossy().to_string()),
            project_name: None,
            author: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Enterprise"), "should mention Enterprise track");
    assert!(text.contains("security.md"), "should list security doc");
    assert!(text.contains("devops.md"), "should list devops doc");

    // Verify extra enterprise files
    assert!(tmp.path().join("_bmad-output/planning-artifacts/security.md").is_file());
    assert!(tmp.path().join("_bmad-output/planning-artifacts/devops.md").is_file());
}

#[tokio::test]
async fn scaffold_unknown_track_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let srv = server();
    let result = srv
        .bmad_scaffold(Parameters(ScaffoldRequest {
            track: "nonexistent_track".to_string(),
            project_dir: Some(tmp.path().to_string_lossy().to_string()),
            project_name: None,
            author: None,
            output_format: None,
        }))
        .await;
    assert!(result.is_err(), "should return error for unknown track");
}

#[tokio::test]
async fn scaffold_nonexistent_dir_returns_error() {
    let srv = server();
    let result = srv
        .bmad_scaffold(Parameters(ScaffoldRequest {
            track: "bmad_method".to_string(),
            project_dir: Some("/nonexistent/scaffold/test/dir".to_string()),
            project_name: None,
            author: None,
            output_format: None,
        }))
        .await;
    assert!(result.is_err(), "should return error for nonexistent dir");
}

#[tokio::test]
async fn scaffold_then_project_state_detects_files() {
    let tmp = tempfile::tempdir().unwrap();
    let srv = server();

    // Scaffold first
    let scaffold_result = srv
        .bmad_scaffold(Parameters(ScaffoldRequest {
            track: "bmad_method".to_string(),
            project_dir: Some(tmp.path().to_string_lossy().to_string()),
            project_name: None,
            author: None,
            output_format: None,
        }))
        .await;
    assert!(scaffold_result.is_ok(), "scaffold should succeed");

    // Then check project state
    let state_result = srv
        .bmad_project_state(Parameters(ProjectStateRequest {
            project_path: tmp.path().to_string_lossy().to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(state_result);
    assert!(text.contains("BMad installed: yes"), "should detect BMad install");
    assert!(text.contains("PRD: found"), "should detect PRD");
    assert!(text.contains("Architecture: found"), "should detect architecture");
    assert!(text.contains("1 epic file(s) found"), "should detect 1 epic");
    assert!(text.contains("Project context: found"), "should detect project context");
}

#[tokio::test]
async fn scaffold_track_aliases_work() {
    // Test that various track aliases are accepted
    for track_name in &["quick", "Quick Flow", "bmad", "BMad Method"] {
        let tmp = tempfile::tempdir().unwrap();
        let srv = server();
        let result = srv
            .bmad_scaffold(Parameters(ScaffoldRequest {
                track: track_name.to_string(),
                project_dir: Some(tmp.path().to_string_lossy().to_string()),
                project_name: None,
                author: None,
                output_format: None,
            }))
            .await;
        let text = text_of(result);
        assert!(
            text.contains("BMad Project Scaffolded"),
            "track alias '{track_name}' should work"
        );
    }
}

#[tokio::test]
async fn scaffold_with_template_vars_substitutes_into_files() {
    let tmp = tempfile::tempdir().unwrap();
    let srv = server();
    let result = srv
        .bmad_scaffold(Parameters(ScaffoldRequest {
            track: "bmad_method".to_string(),
            project_dir: Some(tmp.path().to_string_lossy().to_string()),
            project_name: Some("Widget App".to_string()),
            author: Some("Bob".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("BMad Project Scaffolded"), "should scaffold successfully");

    // Verify template vars were substituted in generated files
    let ctx = std::fs::read_to_string(
        tmp.path().join("_bmad-output/project-context.md"),
    )
    .unwrap();
    assert!(ctx.contains("Widget App"), "project_name should be substituted");
    assert!(ctx.contains("Bob"), "author should be substituted");
    assert!(!ctx.contains("{{project_name}}"), "placeholder should be replaced");

    let prd = std::fs::read_to_string(
        tmp.path().join("_bmad-output/planning-artifacts/PRD.md"),
    )
    .unwrap();
    assert!(prd.contains("Widget App"), "project_name should appear in PRD");
}

// =========================================================================
// JSON output mode tests (output_format: "json")
// =========================================================================

fn json_format() -> Option<String> {
    Some("json".to_string())
}

#[tokio::test]
async fn json_get_workflow_valid() {
    let srv = server();
    let result = srv
        .bmad_get_workflow(Parameters(GetWorkflowRequest {
            workflow_id: "bmad-create-prd".to_string(),
            output_format: json_format(),
        }))
        .await;
    let text = text_of(result);
    let v: serde_json::Value = serde_json::from_str(&text).expect("should be valid JSON");
    assert_eq!(v["id"], "bmad-create-prd");
    assert!(v["phase"].as_str().unwrap().contains("Planning"));
    assert!(v["agent"].as_str().is_some());
    assert!(v["produces"].as_str().is_some());
}

#[tokio::test]
async fn json_get_workflow_not_found() {
    let srv = server();
    let result = srv
        .bmad_get_workflow(Parameters(GetWorkflowRequest {
            workflow_id: "nonexistent".to_string(),
            output_format: json_format(),
        }))
        .await;
    let text = text_of(result);
    let v: serde_json::Value = serde_json::from_str(&text).expect("should be valid JSON");
    assert!(v["error"].as_str().unwrap().contains("not found"));
    assert!(!v["available_workflows"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn json_get_next_steps() {
    let srv = server();
    let result = srv
        .bmad_get_next_steps(Parameters(GetNextStepsRequest {
            phase: "Analysis".to_string(),
            output_format: json_format(),
        }))
        .await;
    let text = text_of(result);
    let v: serde_json::Value = serde_json::from_str(&text).expect("should be valid JSON");
    assert_eq!(v["phase"], "Analysis");
    assert!(v["phase_number"].as_u64().is_some());
    let steps = v["steps"].as_array().unwrap();
    assert!(!steps.is_empty());
    assert!(steps[0]["workflow_id"].as_str().is_some());
}

#[tokio::test]
async fn json_check_readiness() {
    let srv = server();
    let result = srv
        .bmad_check_readiness(Parameters(CheckReadinessRequest {
            project_state: "PRD done, architecture done, epics created".to_string(),
            track: None,
            output_format: json_format(),
        }))
        .await;
    let text = text_of(result);
    let v: serde_json::Value = serde_json::from_str(&text).expect("should be valid JSON");
    assert_eq!(v["ready"], true);
    assert!(v["track"].as_str().is_some());
    assert!(v["missing_artifacts"].as_array().unwrap().is_empty());
    assert!(v["next_action"].as_str().is_some());
}

#[tokio::test]
async fn json_list_agents() {
    let srv = server();
    let result = srv
        .bmad_list_agents(Parameters(ListAgentsRequest {
            phase: None,
            output_format: json_format(),
        }))
        .await;
    let text = text_of(result);
    let v: serde_json::Value = serde_json::from_str(&text).expect("should be valid JSON");
    assert_eq!(v["total"], 9);
    let agents = v["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 9);
    assert!(agents[0]["skill_id"].as_str().is_some());
    assert!(agents[0]["name"].as_str().is_some());
}

#[tokio::test]
async fn json_agent_info() {
    let srv = server();
    let result = srv
        .bmad_agent_info(Parameters(AgentInfoRequest {
            agent_id: "bmad-dev".to_string(),
            output_format: json_format(),
        }))
        .await;
    let text = text_of(result);
    let v: serde_json::Value = serde_json::from_str(&text).expect("should be valid JSON");
    assert_eq!(v["skill_id"], "bmad-dev");
    assert!(v["name"].as_str().unwrap().contains("Developer"));
    assert!(v["persona"].as_str().unwrap().contains("Amelia"));
    let workflows = v["workflows"].as_array().unwrap();
    assert!(!workflows.is_empty());
    assert!(workflows[0]["id"].as_str().is_some());
}

#[tokio::test]
async fn json_index_status() {
    let srv = server();
    let result = srv
        .bmad_index_status(Parameters(IndexStatusRequest {
            output_format: json_format(),
        }))
        .await;
    let text = text_of(result);
    let v: serde_json::Value = serde_json::from_str(&text).expect("should be valid JSON");
    assert_eq!(v["doc_source"], "embedded");
    assert!(v["total_workflows"].as_u64().unwrap() >= 18);
    assert_eq!(v["total_agents"].as_u64().unwrap(), 9);
    assert!(v["doc_byte_size"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn json_default_is_markdown() {
    let srv = server();
    // output_format: None should return markdown, not JSON
    let result = srv
        .bmad_get_workflow(Parameters(GetWorkflowRequest {
            workflow_id: "bmad-create-prd".to_string(),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    // Markdown output should NOT parse as JSON with expected fields
    assert!(
        serde_json::from_str::<serde_json::Value>(&text).is_err()
            || !text.starts_with('{'),
        "default output should be markdown, not JSON"
    );
    assert!(text.contains("bmad-create-prd"), "should still contain workflow id");
}

// =========================================================================
// MCP Resource endpoints (integration tests via duplex transport)
// =========================================================================

/// Spin up a BmadServer over a duplex transport and return the client handle.
async fn resource_client() -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    let index = Arc::new(RwLock::new(BmadIndex::build()));
    let (server_transport, client_transport) = tokio::io::duplex(65536);
    tokio::spawn(async move {
        let server = BmadServer::new(index);
        let svc = server.serve(server_transport).await.unwrap();
        svc.waiting().await.unwrap();
    });
    ().serve(client_transport).await.unwrap()
}

#[tokio::test]
async fn resource_list_contains_docs_and_phases() {
    let client = resource_client().await;
    let result = client.list_all_resources().await.unwrap();

    // Should contain bmad://docs
    assert!(
        result.iter().any(|r| r.uri == "bmad://docs"),
        "should list bmad://docs resource"
    );

    // Should contain all 4 phases
    for phase in ["analysis", "planning", "solutioning", "implementation"] {
        let uri = format!("bmad://phases/{phase}");
        assert!(
            result.iter().any(|r| r.uri == uri),
            "should list phase resource: {uri}"
        );
    }

    // Should contain all 3 tracks
    for track in ["quick-flow", "bmad-method", "enterprise"] {
        let uri = format!("bmad://tracks/{track}");
        assert!(
            result.iter().any(|r| r.uri == uri),
            "should list track resource: {uri}"
        );
    }

    // Should contain workflow resources
    assert!(
        result.iter().any(|r| r.uri == "bmad://workflows/bmad-create-prd"),
        "should list workflow resources"
    );

    // Should contain agent resources
    assert!(
        result.iter().any(|r| r.uri.starts_with("bmad://agents/")),
        "should list agent resources"
    );

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn resource_list_templates() {
    let client = resource_client().await;
    let result = client
        .list_resource_templates(None)
        .await
        .unwrap();

    assert_eq!(result.resource_templates.len(), 4, "should have 4 resource templates");

    let uris: Vec<&str> = result
        .resource_templates
        .iter()
        .map(|t| t.uri_template.as_str())
        .collect();
    assert!(uris.contains(&"bmad://phases/{phase}"));
    assert!(uris.contains(&"bmad://workflows/{workflow_id}"));
    assert!(uris.contains(&"bmad://agents/{agent_id}"));
    assert!(uris.contains(&"bmad://tracks/{track}"));

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn resource_read_docs() {
    let client = resource_client().await;
    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParams::new("bmad://docs"))
        .await
        .unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { uri, text, .. } => {
            assert_eq!(uri, "bmad://docs");
            assert!(!text.is_empty(), "docs content should not be empty");
            assert!(text.contains("BMad"), "docs should mention BMad");
        }
        _ => panic!("expected text resource contents"),
    }

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn resource_read_phase() {
    let client = resource_client().await;
    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParams::new("bmad://phases/planning"))
        .await
        .unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => {
            assert!(text.contains("Planning"), "should describe Planning phase");
            assert!(text.contains("Workflows"), "should list workflows");
        }
        _ => panic!("expected text resource contents"),
    }

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn resource_read_workflow() {
    let client = resource_client().await;
    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParams::new("bmad://workflows/bmad-create-prd"))
        .await
        .unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => {
            assert!(text.contains("bmad-create-prd"), "should contain workflow id");
            assert!(text.contains("Phase"), "should mention phase");
            assert!(text.contains("Agent"), "should mention agent");
        }
        _ => panic!("expected text resource contents"),
    }

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn resource_read_agent() {
    let client = resource_client().await;
    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParams::new("bmad://agents/bmad-pm"))
        .await
        .unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => {
            assert!(text.contains("bmad-pm"), "should contain agent skill id");
            assert!(text.contains("Workflows"), "should list workflows");
        }
        _ => panic!("expected text resource contents"),
    }

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn resource_read_track() {
    let client = resource_client().await;
    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParams::new("bmad://tracks/bmad-method"))
        .await
        .unwrap();

    assert_eq!(result.contents.len(), 1);
    match &result.contents[0] {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => {
            assert!(text.contains("BMad Method"), "should describe BMad Method track");
            assert!(text.contains("Phases"), "should list phases");
            assert!(text.contains("Workflows"), "should list workflows");
        }
        _ => panic!("expected text resource contents"),
    }

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn resource_read_unknown_returns_error() {
    let client = resource_client().await;
    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParams::new("bmad://nonexistent/foo"))
        .await;

    assert!(result.is_err(), "reading unknown URI should return error");

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn resource_read_unknown_workflow_returns_error() {
    let client = resource_client().await;
    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParams::new("bmad://workflows/nonexistent-workflow"))
        .await;

    assert!(result.is_err(), "reading unknown workflow should return error");

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn resource_subscribe_and_unsubscribe() {
    let client = resource_client().await;

    // Subscribe should succeed
    client
        .subscribe(rmcp::model::SubscribeRequestParams::new("bmad://docs"))
        .await
        .unwrap();

    // Unsubscribe should succeed
    client
        .unsubscribe(rmcp::model::UnsubscribeRequestParams::new("bmad://docs"))
        .await
        .unwrap();

    client.cancel().await.unwrap();
}

// =========================================================================
// bmad_run_workflow
// =========================================================================

#[tokio::test]
async fn run_workflow_start_creates_session() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("_bmad")).unwrap();

    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "start".to_string(),
            workflow_id: Some("bmad-create-prd".to_string()),
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Workflow Started"), "should show started message: {text}");
    assert!(text.contains("Step 1/4"), "should show step 1 of 4: {text}");
    assert!(text.contains("Gather Requirements"), "should show first step title: {text}");
}

#[tokio::test]
async fn run_workflow_start_json_output() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("_bmad")).unwrap();

    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "start".to_string(),
            workflow_id: Some("bmad-create-prd".to_string()),
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: None,
            output_format: Some("json".to_string()),
        }))
        .await;
    let text = text_of(result);
    let json: serde_json::Value = serde_json::from_str(&text).expect("should parse as JSON");
    assert_eq!(json["action"], "start");
    assert_eq!(json["total_steps"], 4);
    assert_eq!(json["completed"], false);
    assert!(json["step"].is_object());
}

#[tokio::test]
async fn run_workflow_next_advances_step() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("_bmad")).unwrap();

    // Start
    srv.bmad_run_workflow(Parameters(RunWorkflowRequest {
        action: "start".to_string(),
        workflow_id: Some("bmad-create-story".to_string()),
        project_dir: dir.path().to_string_lossy().to_string(),
        step_result: None,
        output_format: None,
    }))
    .await
    .unwrap();

    // Next (step 1 -> step 2)
    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "next".to_string(),
            workflow_id: Some("bmad-create-story".to_string()),
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: Some("selected story".to_string()),
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Step 2/2"), "should advance to step 2: {text}");
}

#[tokio::test]
async fn run_workflow_next_completes_workflow() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("_bmad")).unwrap();

    // Start bmad-create-story (2 steps)
    srv.bmad_run_workflow(Parameters(RunWorkflowRequest {
        action: "start".to_string(),
        workflow_id: Some("bmad-create-story".to_string()),
        project_dir: dir.path().to_string_lossy().to_string(),
        step_result: None,
        output_format: None,
    }))
    .await
    .unwrap();

    // Advance step 1
    srv.bmad_run_workflow(Parameters(RunWorkflowRequest {
        action: "next".to_string(),
        workflow_id: Some("bmad-create-story".to_string()),
        project_dir: dir.path().to_string_lossy().to_string(),
        step_result: None,
        output_format: None,
    }))
    .await
    .unwrap();

    // Advance step 2 -> completed
    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "next".to_string(),
            workflow_id: Some("bmad-create-story".to_string()),
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("Workflow Completed"), "should show completed: {text}");
    assert!(
        text.contains("bmad-dev-story"),
        "should suggest next workflow: {text}"
    );
}

#[tokio::test]
async fn run_workflow_status_shows_progress() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("_bmad")).unwrap();

    // Start
    srv.bmad_run_workflow(Parameters(RunWorkflowRequest {
        action: "start".to_string(),
        workflow_id: Some("bmad-create-prd".to_string()),
        project_dir: dir.path().to_string_lossy().to_string(),
        step_result: None,
        output_format: None,
    }))
    .await
    .unwrap();

    // Status
    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "status".to_string(),
            workflow_id: Some("bmad-create-prd".to_string()),
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(text.contains("In Progress"), "should show in progress: {text}");
    assert!(text.contains("0/4"), "should show 0/4 steps: {text}");
}

#[tokio::test]
async fn run_workflow_unknown_action_returns_error() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("_bmad")).unwrap();

    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "invalid".to_string(),
            workflow_id: Some("bmad-create-prd".to_string()),
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: None,
            output_format: None,
        }))
        .await;
    assert!(result.is_err(), "unknown action should return error");
}

#[tokio::test]
async fn run_workflow_start_missing_workflow_id_returns_error() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("_bmad")).unwrap();

    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "start".to_string(),
            workflow_id: None,
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: None,
            output_format: None,
        }))
        .await;
    assert!(result.is_err(), "start without workflow_id should error");
}

#[tokio::test]
async fn run_workflow_start_unknown_workflow_returns_error() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("_bmad")).unwrap();

    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "start".to_string(),
            workflow_id: Some("nonexistent".to_string()),
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: None,
            output_format: None,
        }))
        .await;
    assert!(result.is_err(), "unknown workflow should error");
}

#[tokio::test]
async fn run_workflow_next_without_session_returns_error() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("_bmad")).unwrap();

    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "next".to_string(),
            workflow_id: Some("bmad-create-prd".to_string()),
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: None,
            output_format: None,
        }))
        .await;
    assert!(result.is_err(), "next without session should error");
}

#[tokio::test]
async fn run_workflow_auto_detects_completed() {
    let srv = server();
    let dir = tempfile::tempdir().unwrap();
    let bmad_dir = dir.path().join("_bmad");
    std::fs::create_dir_all(&bmad_dir).unwrap();
    let planning = dir.path().join("_bmad-output/planning-artifacts");
    std::fs::create_dir_all(&planning).unwrap();
    std::fs::write(planning.join("PRD.md"), "# PRD
Test content").unwrap();

    let result = srv
        .bmad_run_workflow(Parameters(RunWorkflowRequest {
            action: "start".to_string(),
            workflow_id: Some("bmad-create-prd".to_string()),
            project_dir: dir.path().to_string_lossy().to_string(),
            step_result: None,
            output_format: None,
        }))
        .await;
    let text = text_of(result);
    assert!(
        text.contains("Already completed"),
        "should detect existing PRD as completed: {text}"
    );
}
