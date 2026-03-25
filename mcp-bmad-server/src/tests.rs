use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use tokio::sync::RwLock;

use crate::bmad_index::BmadIndex;
use crate::{
    AgentInfoRequest, BmadServer, CheckReadinessRequest, GetNextStepsRequest,
    GetTrackWorkflowsRequest, GetWorkflowRequest, HelpRequest, IndexStatusRequest,
    ListAgentsRequest, NextStepRequest, ProjectStateRequest, SprintGuideRequest,
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
        .bmad_list_agents(Parameters(ListAgentsRequest { phase: None }))
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
        .bmad_index_status(Parameters(IndexStatusRequest {}))
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
        .bmad_index_status(Parameters(IndexStatusRequest {}))
        .await;
    let text = text_of(result);
    // Should have at least 18 workflows and 9 agents
    assert!(text.contains("18") || text.contains("19") || text.contains("20"),
        "should show workflow count around 18+: {text}");
    assert!(text.contains("9"), "should show 9 agents");
}
