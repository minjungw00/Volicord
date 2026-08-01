//! Current human-presentation semantics for Doctor checks.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DoctorCheckGroup {
    RuntimeAndBuild,
    IntegrationControl,
    GuardAndHookState,
    ProjectIntegration,
    CommandAvailability,
    InventoryAndOptionalDiagnostics,
}

impl DoctorCheckGroup {
    pub(super) const ALL: [Self; 6] = [
        Self::RuntimeAndBuild,
        Self::IntegrationControl,
        Self::GuardAndHookState,
        Self::ProjectIntegration,
        Self::CommandAvailability,
        Self::InventoryAndOptionalDiagnostics,
    ];

    pub(super) const fn human_title(self) -> &'static str {
        match self {
            Self::RuntimeAndBuild => "Runtime and build",
            Self::IntegrationControl => "Integration control",
            Self::GuardAndHookState => "Guard and Hook state",
            Self::ProjectIntegration => "Project integration",
            Self::CommandAvailability => "Command availability",
            Self::InventoryAndOptionalDiagnostics => "Inventory and optional diagnostics",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DoctorDetailProjection {
    pub(super) path: &'static str,
    pub(super) human_label: &'static str,
}

impl DoctorDetailProjection {
    const fn new(path: &'static str, human_label: &'static str) -> Self {
        Self { path, human_label }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DoctorHealthyProjection {
    SummaryOnly,
    SelectedDetails(&'static [DoctorDetailProjection]),
    BuildIdentity,
    RegistrySchema,
    GuardFiles,
    CommandAggregate,
    HostDetection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DoctorNonSuccessProjection {
    FullDetails,
    BuildIdentity,
    RegistrySchema,
    GuardFiles,
    CommandCheck,
    HostDetection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DoctorCheckPresentation {
    pub(super) check_id: &'static str,
    pub(super) human_title: &'static str,
    pub(super) group: DoctorCheckGroup,
    pub(super) healthy_projection: DoctorHealthyProjection,
    pub(super) non_success_projection: DoctorNonSuccessProjection,
    pub(super) related_finding_codes: &'static [&'static str],
}

const RUNTIME_HOME_DETAILS: &[DoctorDetailProjection] =
    &[DoctorDetailProjection::new("path", "Path")];
const REGISTRY_DETAILS: &[DoctorDetailProjection] = &[DoctorDetailProjection::new("path", "Path")];
const INSTALLATION_PROFILE_DETAILS: &[DoctorDetailProjection] = &[
    DoctorDetailProjection::new("default_connection_mode", "Default connection mode"),
    DoctorDetailProjection::new("bin_dir", "Installation bin directory"),
];
const CONTROL_SURFACE_DETAILS: &[DoctorDetailProjection] = &[
    DoctorDetailProjection::new("selected_profile", "Selected profile"),
    DoctorDetailProjection::new("control_surface.host_hooks_active", "Host Hooks active"),
    DoctorDetailProjection::new(
        "control_surface.cooperative_pre_tool_warning_available",
        "Cooperative pre-tool warning available",
    ),
    DoctorDetailProjection::new(
        "control_surface.cooperative_pre_tool_denial_available",
        "Cooperative pre-tool denial available",
    ),
    DoctorDetailProjection::new(
        "control_surface.unrecorded_changes_detectable",
        "Unrecorded Changes detectable",
    ),
    DoctorDetailProjection::new(
        "control_surface.actor_identity_provable",
        "Actor identity provable",
    ),
    DoctorDetailProjection::new("control_surface.os_enforced", "OS enforced"),
];
const GUARD_OBSERVATION_DETAILS: &[DoctorDetailProjection] = &[
    DoctorDetailProjection::new("observed", "Current observations"),
    DoctorDetailProjection::new("installations", "Guard installations"),
    DoctorDetailProjection::new("incompatible_events", "Incompatible events"),
    DoctorDetailProjection::new("prompt_capture_configured", "Prompt capture configured"),
    DoctorDetailProjection::new("prompt_capture_observed", "Prompt capture observed"),
];
const PERSONAL_GIT_DETAILS: &[DoctorDetailProjection] = &[
    DoctorDetailProjection::new("connected_project_count", "Connected projects"),
    DoctorDetailProjection::new(
        "effective_personal_project_count",
        "Personal integration projects",
    ),
    DoctorDetailProjection::new("audited_project_count", "Audited projects"),
    DoctorDetailProjection::new("truncated", "Audit truncated"),
];
const INTEGRATION_INTENT_DETAILS: &[DoctorDetailProjection] = &[
    DoctorDetailProjection::new("connected_project_count", "Connected projects"),
    DoctorDetailProjection::new("audited_project_count", "Audited projects"),
    DoctorDetailProjection::new("truncated", "Audit truncated"),
];
const PROJECT_POLICY_DETAILS: &[DoctorDetailProjection] = &[
    DoctorDetailProjection::new("project_count", "Connected projects"),
    DoctorDetailProjection::new("scan_state", "Scan state"),
];
const REGISTRY_COUNT_DETAILS: &[DoctorDetailProjection] = &[
    DoctorDetailProjection::new("projects", "Projects"),
    DoctorDetailProjection::new("connections", "Connections"),
    DoctorDetailProjection::new("guard_installations", "Guard installations"),
];

const NO_FINDING_CODES: &[&str] = &[];
const BUILD_FINDING_CODES: &[&str] = &[
    "installation.build_identity.unavailable",
    "installation.build_source.not_reproducible",
];
const CLI_COMMAND_FINDING_CODES: &[&str] = &[
    "installation.executable.missing",
    "installation.executable.not_runnable",
];
const INSTALLED_BUILD_FINDING_CODES: &[&str] = &["installation.managed_config.inconsistent"];

pub(super) const CURRENT_DOCTOR_CHECK_PRESENTATIONS: &[DoctorCheckPresentation] = &[
    DoctorCheckPresentation {
        check_id: "build_identity",
        human_title: "Build identity",
        group: DoctorCheckGroup::RuntimeAndBuild,
        healthy_projection: DoctorHealthyProjection::BuildIdentity,
        non_success_projection: DoctorNonSuccessProjection::BuildIdentity,
        related_finding_codes: BUILD_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "runtime_home_access",
        human_title: "Runtime Home access",
        group: DoctorCheckGroup::RuntimeAndBuild,
        healthy_projection: DoctorHealthyProjection::SelectedDetails(RUNTIME_HOME_DETAILS),
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "registry",
        human_title: "Runtime Home registry",
        group: DoctorCheckGroup::RuntimeAndBuild,
        healthy_projection: DoctorHealthyProjection::SelectedDetails(REGISTRY_DETAILS),
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "registry_schema",
        human_title: "Registry schema",
        group: DoctorCheckGroup::RuntimeAndBuild,
        healthy_projection: DoctorHealthyProjection::RegistrySchema,
        non_success_projection: DoctorNonSuccessProjection::RegistrySchema,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "installation_profile",
        human_title: "Installation profile",
        group: DoctorCheckGroup::RuntimeAndBuild,
        healthy_projection: DoctorHealthyProjection::SelectedDetails(INSTALLATION_PROFILE_DETAILS),
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "installed_build_configuration",
        human_title: "Installed build configuration",
        group: DoctorCheckGroup::RuntimeAndBuild,
        healthy_projection: DoctorHealthyProjection::SummaryOnly,
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: INSTALLED_BUILD_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "control_surface",
        human_title: "Integration control surface",
        group: DoctorCheckGroup::IntegrationControl,
        healthy_projection: DoctorHealthyProjection::SelectedDetails(CONTROL_SURFACE_DETAILS),
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "guard_files",
        human_title: "Guard files",
        group: DoctorCheckGroup::GuardAndHookState,
        healthy_projection: DoctorHealthyProjection::GuardFiles,
        non_success_projection: DoctorNonSuccessProjection::GuardFiles,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "guard_observation",
        human_title: "Guard observation",
        group: DoctorCheckGroup::GuardAndHookState,
        healthy_projection: DoctorHealthyProjection::SelectedDetails(GUARD_OBSERVATION_DETAILS),
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "personal_local_git_tracking",
        human_title: "Personal integration Git tracking",
        group: DoctorCheckGroup::ProjectIntegration,
        healthy_projection: DoctorHealthyProjection::SelectedDetails(PERSONAL_GIT_DETAILS),
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "integration_intent_drift",
        human_title: "Integration intent",
        group: DoctorCheckGroup::ProjectIntegration,
        healthy_projection: DoctorHealthyProjection::SelectedDetails(INTEGRATION_INTENT_DETAILS),
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "project_policy_authority",
        human_title: "Project policy authority",
        group: DoctorCheckGroup::ProjectIntegration,
        healthy_projection: DoctorHealthyProjection::SelectedDetails(PROJECT_POLICY_DETAILS),
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "volicord_command",
        human_title: "CLI command",
        group: DoctorCheckGroup::CommandAvailability,
        healthy_projection: DoctorHealthyProjection::CommandAggregate,
        non_success_projection: DoctorNonSuccessProjection::CommandCheck,
        related_finding_codes: CLI_COMMAND_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "volicord_mcp_command",
        human_title: "MCP launch command",
        group: DoctorCheckGroup::CommandAvailability,
        healthy_projection: DoctorHealthyProjection::CommandAggregate,
        non_success_projection: DoctorNonSuccessProjection::CommandCheck,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "volicord_command_availability",
        human_title: "CLI PATH resolution",
        group: DoctorCheckGroup::CommandAvailability,
        healthy_projection: DoctorHealthyProjection::CommandAggregate,
        non_success_projection: DoctorNonSuccessProjection::CommandCheck,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "volicord_mcp_command_availability",
        human_title: "MCP PATH resolution",
        group: DoctorCheckGroup::CommandAvailability,
        healthy_projection: DoctorHealthyProjection::CommandAggregate,
        non_success_projection: DoctorNonSuccessProjection::CommandCheck,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "path_or_shim",
        human_title: "PATH and command links",
        group: DoctorCheckGroup::CommandAvailability,
        healthy_projection: DoctorHealthyProjection::CommandAggregate,
        non_success_projection: DoctorNonSuccessProjection::CommandCheck,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "registry_counts",
        human_title: "Registry inventory",
        group: DoctorCheckGroup::InventoryAndOptionalDiagnostics,
        healthy_projection: DoctorHealthyProjection::SelectedDetails(REGISTRY_COUNT_DETAILS),
        non_success_projection: DoctorNonSuccessProjection::FullDetails,
        related_finding_codes: NO_FINDING_CODES,
    },
    DoctorCheckPresentation {
        check_id: "host_detection",
        human_title: "Host detection",
        group: DoctorCheckGroup::InventoryAndOptionalDiagnostics,
        healthy_projection: DoctorHealthyProjection::HostDetection,
        non_success_projection: DoctorNonSuccessProjection::HostDetection,
        related_finding_codes: NO_FINDING_CODES,
    },
];

pub(super) fn doctor_check_presentation(
    check_id: &str,
) -> Option<&'static DoctorCheckPresentation> {
    CURRENT_DOCTOR_CHECK_PRESENTATIONS
        .iter()
        .find(|presentation| presentation.check_id == check_id)
}
