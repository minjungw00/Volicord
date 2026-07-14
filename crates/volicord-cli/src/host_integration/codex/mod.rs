mod adapter;
mod config;
mod executable;
mod identity;
mod trust;

use std::path::{Path, PathBuf};

use crate::host_integration::HostCapabilities;

pub use adapter::{CodexAdapter, CodexEnvironment, CodexExistingPlanRequest};
pub(crate) use identity::{
    managed_entry_from_item_for_diagnostics, managed_identity_evaluation_for_plan,
};
pub(crate) use trust::project_trust_diagnostic;

const VOLICORD_MCP_LAUNCH: &str = "VOLICORD_MCP_LAUNCH";
const VOLICORD_MCP_HOST: &str = "VOLICORD_MCP_HOST";
const VOLICORD_MCP_CONNECTION_ID: &str = "VOLICORD_MCP_CONNECTION_ID";
const VOLICORD_MCP_PROJECT_ID: &str = "VOLICORD_MCP_PROJECT_ID";
const MANAGED_HOST_LAUNCH_VALUE: &str = "managed_host";
const CODEX_HOST_VALUE: &str = "codex";
const CODEX_TOOL_APPROVAL_OVERLAY_KIND: &str = "codex_tool_approval";

pub fn capabilities() -> HostCapabilities {
    HostCapabilities {
        stdio_mcp: true,
        http_mcp: false,
        session_start_hook: true,
        pre_tool_hook: true,
        post_tool_hook: true,
        user_prompt_submit_hook: true,
        stop_hook: true,
        rule_file_support: true,
        project_local_configuration: true,
    }
}

pub(crate) fn project_hooks_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".codex").join("hooks.json")
}

pub(crate) fn project_rule_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join(".codex")
        .join("rules")
        .join("volicord.rules")
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::host_integration::{
        capability_status::REVIEWED_CODEX_HOST_VERSION,
        claude_code::{CommandInvocation, CommandOutput, CommandRunner},
        managed_fingerprint,
        verification::{HostExecutableStatus, ManagedConfigStatus, ProjectTrustStatus},
        ConnectionIntent, HostAdapter, HostConfigError, HostConflictKind, HostKind,
        HostPlanRequest, HostRemoveRequest, HostScope, HostTarget, InstallationProfile,
        ManagedServerEntry, PlannedChange, ProjectContext, DEFAULT_SERVER_NAME,
    };
    use volicord_mcp::RepositoryDiscoveryHost;

    use super::*;

    #[test]
    fn user_config_path_defaults_to_home_codex() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-home-default")?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir.clone()),
            codex_home: None,
            path: None,
        });

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;

        assert_eq!(
            plan.target,
            HostTarget::File(dir.join(".codex").join("config.toml"))
        );
        Ok(())
    }

    #[test]
    fn user_config_path_honors_codex_home() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-home-override")?;
        let codex_home = dir.join("custom-codex");
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir),
            codex_home: Some(codex_home.clone()),
            path: None,
        });

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;

        assert_eq!(
            plan.target,
            HostTarget::File(codex_home.join("config.toml"))
        );
        Ok(())
    }

    #[test]
    fn project_config_path_is_repository_scoped() -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-project")?;
        let adapter = CodexAdapter::new(CodexEnvironment::default());

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;

        assert_eq!(
            plan.target,
            HostTarget::File(repo.join(".codex").join("config.toml"))
        );
        assert!(plan.user_actions.is_empty());
        Ok(())
    }

    #[test]
    fn codex_project_trust_reads_trusted() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-trust-trusted")?;
        let repo = dir.join("product");
        fs::create_dir_all(&repo)?;
        let codex_home = dir.join("codex-home");
        write_project_trust(&codex_home, &repo, "trusted")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Trusted);
        Ok(())
    }

    #[test]
    fn codex_project_trust_reads_untrusted() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-trust-untrusted")?;
        let repo = dir.join("product");
        fs::create_dir_all(&repo)?;
        let codex_home = dir.join("codex-home");
        write_project_trust(&codex_home, &repo, "untrusted")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Untrusted);
        Ok(())
    }

    #[test]
    fn codex_project_trust_missing_project_entry_is_missing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-trust-missing")?;
        let repo = dir.join("product");
        let other = dir.join("other");
        fs::create_dir_all(&repo)?;
        fs::create_dir_all(&other)?;
        let codex_home = dir.join("codex-home");
        write_project_trust(&codex_home, &other, "trusted")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Missing);
        Ok(())
    }

    #[test]
    fn codex_project_trust_malformed_config_is_malformed() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = temp_dir("codex-trust-malformed")?;
        let repo = dir.join("product");
        fs::create_dir_all(&repo)?;
        let codex_home = dir.join("codex-home");
        fs::create_dir_all(&codex_home)?;
        fs::write(codex_home.join("config.toml"), "[projects.\n")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Malformed);
        Ok(())
    }

    #[test]
    fn codex_project_trust_respects_codex_home() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-trust-codex-home")?;
        let repo = dir.join("product");
        fs::create_dir_all(&repo)?;
        let home = dir.join("home");
        let default_codex_home = home.join(".codex");
        let codex_home = dir.join("codex-home");
        write_project_trust(&default_codex_home, &repo, "untrusted")?;
        write_project_trust(&codex_home, &repo, "trusted")?;

        let trust = project_trust_diagnostic(
            &CodexEnvironment {
                home: Some(home),
                codex_home: Some(codex_home),
                path: None,
            },
            &repo,
        );

        assert_eq!(trust.status, ProjectTrustStatus::Trusted);
        Ok(())
    }

    #[test]
    fn intent_mapping_rejects_codex_global() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-intent")?;
        let repo = temp_dir("codex-intent-repo")?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir.clone()),
            codex_home: None,
            path: None,
        });

        let personal = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        let shared = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        let global = adapter
            .plan(HostPlanRequest {
                connection_intent: ConnectionIntent::Global,
                ..request(HostScope::User, None, Path::new("/bin/volicord"))
            })
            .expect_err("Codex global intent should be unsupported");

        assert_eq!(personal.host_scope, HostScope::User);
        assert_eq!(shared.host_scope, HostScope::Project);
        assert!(matches!(global, HostConfigError::Conflict(_)));
        assert!(global
            .to_string()
            .contains("supported connection intents: personal, shared"));
        Ok(())
    }

    #[test]
    fn existing_plan_uses_stored_target_without_ambient_discovery(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-existing-target")?;
        let stored_target = dir.join("stored").join("config.toml");
        let ambient_codex_home = dir.join("ambient");
        fs::create_dir_all(&ambient_codex_home)?;
        fs::write(
            ambient_codex_home.join("config.toml"),
            "[mcp_servers.volicord-existing]\ncommand = \"ambient\"\n",
        )?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: Some(dir.join("home")),
            codex_home: Some(ambient_codex_home),
            path: None,
        });

        let plan = adapter.plan_existing(existing_request(
            HostScope::User,
            &stored_target,
            Path::new("/bin/volicord"),
            Some(Path::new("/runtime")),
        ))?;

        assert_eq!(plan.target, HostTarget::File(stored_target));
        assert_eq!(plan.change, PlannedChange::Noop);
        assert_ne!(plan.fingerprint, "stored-fingerprint");
        assert_eq!(
            plan.entry
                .env
                .get(VOLICORD_MCP_CONNECTION_ID)
                .map(String::as_str),
            Some("int_alpha")
        );
        Ok(())
    }

    #[test]
    fn existing_plan_verification_reports_stored_missing_without_ambient_fallback(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-existing-missing")?;
        let stored_target = dir.join("stored").join("config.toml");
        let ambient_codex_home = dir.join("ambient");
        fs::create_dir_all(&ambient_codex_home)?;
        fs::write(
            ambient_codex_home.join("config.toml"),
            "[mcp_servers.volicord-existing]\ncommand = \"ambient\"\n",
        )?;
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(ambient_codex_home),
            path: Some(dir.join("empty-path").into_os_string()),
        });
        let plan = adapter.plan_existing(existing_request(
            HostScope::User,
            &stored_target,
            Path::new("/bin/volicord"),
            Some(Path::new("/runtime")),
        ))?;

        let verification = adapter.verify(&plan)?;

        assert_eq!(verification.status.as_str(), "missing");
        assert_eq!(verification.managed_config, ManagedConfigStatus::Missing);
        Ok(())
    }

    #[test]
    fn insertion_preserves_comments_and_unrelated_keys() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-preserve")?;
        let codex_home = dir.join("codex");
        fs::create_dir_all(&codex_home)?;
        let target = codex_home.join("config.toml");
        fs::write(
            &target,
            "# keep me\nmodel = \"gpt-5.5\"\n\n[mcp_servers.other]\ncommand = \"other\"\n",
        )?;
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: None,
        });

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&plan)?;
        let text = fs::read_to_string(target)?;

        assert!(text.contains("# keep me"));
        assert!(text.contains("model = \"gpt-5.5\""));
        assert!(text.contains("[mcp_servers.other]"));
        assert!(text.contains("[mcp_servers.volicord]"));
        assert!(text.contains("args = [\"mcp\", \"--stdio\", \"--connection\", \"int_alpha\"]"));
        assert!(text.contains("[mcp_servers.volicord.env]"));
        assert!(text.contains("VOLICORD_MCP_LAUNCH = \"managed_host\""));
        assert!(text.contains("VOLICORD_MCP_HOST = \"codex\""));
        assert!(text.contains("VOLICORD_MCP_CONNECTION_ID = \"int_alpha\""));
        Ok(())
    }

    #[test]
    fn project_config_uses_portable_discovery_without_local_binding_fields(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-project-env")?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&plan)?;
        let text = fs::read_to_string(repo.join(".codex/config.toml"))?;

        assert_eq!(
            plan.entry.args,
            ["mcp", "--stdio", "--discover-repository", "--host", "codex"]
        );
        assert!(plan.entry.env.is_empty());
        assert_eq!(plan.entry.env_vars, ["VOLICORD_HOME"]);
        assert!(text.contains(
            "args = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]"
        ));
        assert!(text.contains("env_vars = [\"VOLICORD_HOME\"]"));
        assert!(!text.contains("[mcp_servers.volicord.env]"));
        assert!(!text.contains("int_alpha"));
        assert!(!text.contains("project_alpha"));
        assert!(!text.contains("/runtime"));
        Ok(())
    }

    #[test]
    fn managed_entry_with_tool_approval_overlay_is_match() -> Result<(), Box<dyn std::error::Error>>
    {
        let repo = temp_dir("codex-project-overlay-match")?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&plan)?;
        let target = repo.join(".codex/config.toml");
        append_tool_approval_overlay(&target, "volicord.intake")?;

        let evaluation = managed_identity_evaluation_for_plan(&plan)?;
        let plan_after_overlay = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;

        assert_eq!(evaluation.status, ManagedConfigStatus::Match);
        let overlay = evaluation
            .host_policy_overlay
            .expect("overlay diagnostic should be present");
        assert!(overlay.present);
        assert!(overlay.accepted);
        assert_eq!(overlay.kind, CODEX_TOOL_APPROVAL_OVERLAY_KIND);
        assert_eq!(overlay.tool_count, 1);
        assert_eq!(overlay.tools, vec!["volicord.intake".to_owned()]);
        assert_eq!(overlay.entries[0].tool, "volicord.intake");
        assert_eq!(overlay.entries[0].approval_mode, "approve");
        assert_eq!(plan_after_overlay.change, PlannedChange::Noop);
        assert!(plan_after_overlay.conflicts.is_empty());
        Ok(())
    }

    #[test]
    fn update_preserves_tool_approval_overlay() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-overlay-preserve")?;
        let codex_home = dir.join("codex");
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home.clone()),
            path: None,
        });
        let first = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&first)?;
        let target = codex_home.join("config.toml");
        append_tool_approval_overlay(&target, "volicord.status")?;

        let update = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&first.fingerprint),
            installation_profile: InstallationProfile {
                volicord_mcp_command: Path::new("/usr/local/bin/volicord"),
                ..request(HostScope::User, None, Path::new("/bin/volicord")).installation_profile
            },
            ..request(HostScope::User, None, Path::new("/bin/volicord"))
        })?;
        assert_eq!(update.change, PlannedChange::Update);
        adapter.apply(&update)?;
        let text = fs::read_to_string(target)?;

        assert!(text.contains("command = \"/usr/local/bin/volicord\""));
        assert!(text.contains("[mcp_servers.volicord.tools.\"volicord.status\"]"));
        assert!(text.contains("approval_mode = \"approve\""));
        Ok(())
    }

    #[test]
    fn project_config_with_nonportable_command_is_unmanaged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-command-changed")?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&plan)?;
        let target = repo.join(".codex/config.toml");
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("command = \"volicord\"", "command = \"other\""),
        )?;

        let status = managed_identity_evaluation_for_plan(&plan)?.status;

        assert_eq!(status, ManagedConfigStatus::Unmanaged);
        Ok(())
    }

    #[test]
    fn project_config_with_wrong_discovery_host_is_unmanaged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-connection-changed")?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&plan)?;
        let target = repo.join(".codex/config.toml");
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("\"codex\"", "\"claude-code\""),
        )?;

        let status = managed_identity_evaluation_for_plan(&plan)?.status;

        assert_eq!(status, ManagedConfigStatus::Unmanaged);
        Ok(())
    }

    #[test]
    fn project_config_with_injected_local_ids_is_unmanaged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-project-changed")?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&plan)?;
        let target = repo.join(".codex/config.toml");
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace(
                "\"codex\"]",
                "\"codex\", \"--connection\", \"int_alpha\", \"--project\", \"project_alpha\"]",
            ),
        )?;

        let status = managed_identity_evaluation_for_plan(&plan)?.status;

        assert_eq!(status, ManagedConfigStatus::Unmanaged);
        Ok(())
    }

    #[test]
    fn owned_table_updates_and_idempotent_reapply() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-update")?;
        let codex_home = dir.join("codex");
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: None,
        });
        let first = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&first)?;

        let second = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&first.fingerprint),
            installation_profile: InstallationProfile {
                volicord_mcp_command: Path::new("/usr/local/bin/volicord"),
                ..request(HostScope::User, None, Path::new("/bin/volicord")).installation_profile
            },
            ..request(HostScope::User, None, Path::new("/bin/volicord"))
        })?;
        assert_eq!(second.change, PlannedChange::Update);
        adapter.apply(&second)?;

        let third = adapter.plan(HostPlanRequest {
            installation_profile: InstallationProfile {
                volicord_mcp_command: Path::new("/usr/local/bin/volicord"),
                ..request(HostScope::User, None, Path::new("/bin/volicord")).installation_profile
            },
            ..request(HostScope::User, None, Path::new("/bin/volicord"))
        })?;
        assert_eq!(third.change, PlannedChange::Noop);
        Ok(())
    }

    #[test]
    fn stored_shared_binding_migrates_once_to_portable_discovery(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-legacy-shared-migration")?;
        fs::create_dir_all(repo.join(".codex"))?;
        let mut legacy = ManagedServerEntry::new_project_bound(
            "int_alpha",
            Some("project_alpha"),
            Path::new("volicord"),
            None,
        );
        legacy.env.insert(
            VOLICORD_MCP_LAUNCH.to_owned(),
            MANAGED_HOST_LAUNCH_VALUE.to_owned(),
        );
        legacy
            .env
            .insert(VOLICORD_MCP_HOST.to_owned(), CODEX_HOST_VALUE.to_owned());
        legacy.env.insert(
            VOLICORD_MCP_CONNECTION_ID.to_owned(),
            "int_alpha".to_owned(),
        );
        legacy.env.insert(
            VOLICORD_MCP_PROJECT_ID.to_owned(),
            "project_alpha".to_owned(),
        );
        let legacy_fingerprint = managed_fingerprint(
            HostKind::Codex,
            HostScope::Project,
            DEFAULT_SERVER_NAME,
            &legacy,
        );
        fs::write(
            repo.join(".codex/config.toml"),
            "[mcp_servers.volicord]\ncommand = \"volicord\"\nargs = [\"mcp\", \"--stdio\", \"--connection\", \"int_alpha\", \"--project\", \"project_alpha\"]\n\n[mcp_servers.volicord.env]\nVOLICORD_MCP_LAUNCH = \"managed_host\"\nVOLICORD_MCP_HOST = \"codex\"\nVOLICORD_MCP_CONNECTION_ID = \"int_alpha\"\nVOLICORD_MCP_PROJECT_ID = \"project_alpha\"\n",
        )?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());

        let migration = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&legacy_fingerprint),
            ..request(HostScope::Project, Some(&repo), Path::new("ignored"))
        })?;
        assert_eq!(migration.change, PlannedChange::Update);
        adapter.apply(&migration)?;

        let migrated = fs::read_to_string(repo.join(".codex/config.toml"))?;
        assert!(migrated.contains("--discover-repository"));
        assert!(migrated.contains("env_vars = [\"VOLICORD_HOME\"]"));
        assert!(!migrated.contains("--connection"));
        assert!(!migrated.contains("[mcp_servers.volicord.env]"));
        let again = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&migration.fingerprint),
            ..request(HostScope::Project, Some(&repo), Path::new("ignored"))
        })?;
        assert_eq!(again.change, PlannedChange::Noop);
        Ok(())
    }

    #[test]
    fn stored_discovery_without_forwarding_migrates_with_its_v1_fingerprint(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-discovery-forwarding-migration")?;
        fs::create_dir_all(repo.join(".codex"))?;
        let mut legacy =
            ManagedServerEntry::new_repository_discovery(RepositoryDiscoveryHost::Codex);
        legacy.env_vars.clear();
        let legacy_fingerprint = managed_fingerprint(
            HostKind::Codex,
            HostScope::Project,
            DEFAULT_SERVER_NAME,
            &legacy,
        );
        fs::write(
            repo.join(".codex/config.toml"),
            "[mcp_servers.volicord]\ncommand = \"volicord\"\nargs = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]\n",
        )?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());

        let migration = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&legacy_fingerprint),
            ..request(HostScope::Project, Some(&repo), Path::new("ignored"))
        })?;
        assert_eq!(migration.change, PlannedChange::Update);
        assert!(migration.conflicts.is_empty());
        adapter.apply(&migration)?;

        let migrated = fs::read_to_string(repo.join(".codex/config.toml"))?;
        assert!(migrated.contains("env_vars = [\"VOLICORD_HOME\"]"));
        let again = adapter.plan(HostPlanRequest {
            expected_fingerprint: Some(&migration.fingerprint),
            ..request(HostScope::Project, Some(&repo), Path::new("ignored"))
        })?;
        assert_eq!(again.change, PlannedChange::Noop);
        Ok(())
    }

    #[test]
    fn project_discovery_rejects_injected_environment_shapes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (name, extra) in [
            (
                "forwarded-secret",
                "env_vars = [\"VOLICORD_HOME\", \"API_TOKEN\"]\n",
            ),
            (
                "literal-home",
                "[mcp_servers.volicord.env]\nVOLICORD_HOME = \"/tmp/injected\"\n",
            ),
        ] {
            let repo = temp_dir(&format!("codex-project-env-reject-{name}"))?;
            fs::create_dir_all(repo.join(".codex"))?;
            fs::write(
                repo.join(".codex/config.toml"),
                format!(
                    "[mcp_servers.volicord]\ncommand = \"volicord\"\nargs = [\"mcp\", \"--stdio\", \"--discover-repository\", \"--host\", \"codex\"]\n{extra}"
                ),
            )?;
            let adapter = CodexAdapter::new(CodexEnvironment::default());

            let plan = adapter.plan(request(
                HostScope::Project,
                Some(&repo),
                Path::new("ignored"),
            ))?;

            assert_eq!(plan.change, PlannedChange::Noop, "{name}");
            assert_eq!(
                plan.conflicts[0].kind,
                HostConflictKind::UnmanagedNameCollision,
                "{name}"
            );
        }
        Ok(())
    }

    #[test]
    fn unmanaged_name_collision_is_reported() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-collision")?;
        let codex_home = dir.join("codex");
        fs::create_dir_all(&codex_home)?;
        fs::write(
            codex_home.join("config.toml"),
            "[mcp_servers.volicord]\ncommand = \"other\"\n",
        )?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: None,
        });

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;

        assert_eq!(
            plan.conflicts[0].kind,
            HostConflictKind::UnmanagedNameCollision
        );
        Ok(())
    }

    #[test]
    fn managed_fingerprint_mismatch_is_reported() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-managed-mismatch")?;
        let codex_home = dir.join("codex");
        fs::create_dir_all(&codex_home)?;
        fs::write(
            codex_home.join("config.toml"),
            "[mcp_servers.volicord]\ncommand = \"/bin/volicord\"\nargs = [\"mcp\", \"--stdio\", \"--connection\", \"other\"]\n\n[mcp_servers.volicord.env]\nVOLICORD_MCP_LAUNCH = \"managed_host\"\nVOLICORD_MCP_HOST = \"codex\"\nVOLICORD_MCP_CONNECTION_ID = \"other\"\n",
        )?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: None,
        });

        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;

        assert_eq!(
            plan.conflicts[0].kind,
            HostConflictKind::FingerprintMismatch
        );
        Ok(())
    }

    #[test]
    fn malformed_toml_is_rejected_without_write() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-malformed")?;
        let codex_home = dir.join("codex");
        fs::create_dir_all(&codex_home)?;
        let target = codex_home.join("config.toml");
        fs::write(&target, "[mcp_servers.\n")?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: None,
        });

        let error = adapter
            .plan(request(HostScope::User, None, Path::new("/bin/volicord")))
            .expect_err("malformed TOML should fail");

        assert!(matches!(error, HostConfigError::Malformed(_)));
        assert_eq!(fs::read_to_string(target)?, "[mcp_servers.\n");
        Ok(())
    }

    #[test]
    fn shared_intent_uses_path_command_and_parent_runtime_home_forwarding(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-project-path")?;
        let adapter = CodexAdapter::new(CodexEnvironment::default());

        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("/personal/target/debug/volicord"),
        ))?;

        assert_eq!(plan.entry.command, "volicord");
        assert_eq!(
            plan.entry.args,
            ["mcp", "--stdio", "--discover-repository", "--host", "codex"]
        );
        assert!(!plan.entry.env.contains_key("VOLICORD_HOME"));
        assert!(plan.entry.env.is_empty());
        assert_eq!(plan.entry.env_vars, ["VOLICORD_HOME"]);
        assert!(!plan.entry.args.iter().any(|arg| arg == "int_alpha"));
        Ok(())
    }

    #[test]
    fn safe_removal_requires_matching_fingerprint() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-remove")?;
        let codex_home = dir.join("codex");
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: None,
        });
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&plan)?;
        let HostTarget::File(target) = plan.target.clone() else {
            unreachable!("codex target");
        };
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("/bin/volicord", "/tmp/manual"),
        )?;

        let error = adapter
            .remove(HostRemoveRequest {
                host_kind: HostKind::Codex,
                connection_intent: plan.connection_intent,
                host_scope: HostScope::User,
                mode: plan.mode.clone(),
                server_name: plan.server_name,
                target: HostTarget::File(target),
                expected_fingerprint: plan.fingerprint,
            })
            .expect_err("manual edits should block removal");

        assert!(matches!(error, HostConfigError::Conflict(_)));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn detect_requires_executable_on_path() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-detect")?;
        let codex_home = dir.join("codex");
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: Some(dir.join("empty").into_os_string()),
        });

        let detection = adapter.detect()?;

        assert!(!detection.available);
        assert_eq!(detection.host_version, None);
        assert!(detection.details.contains("not found on PATH"));
        Ok(())
    }

    #[test]
    fn detect_reports_available_executable() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-detect-available")?;
        let codex_home = dir.join("codex");
        let bin = dir.join("bin");
        write_fake_codex_file(&bin)?;
        let adapter = CodexAdapter::with_runner(
            CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: Some(bin.into_os_string()),
            },
            FakeRunner::new(vec![Ok(ok_output())]),
        );

        let detection = adapter.detect()?;

        assert!(detection.available);
        assert_eq!(
            detection.host_version.as_deref(),
            Some(REVIEWED_CODEX_HOST_VERSION)
        );
        assert!(detection.details.contains("codex --version"));
        assert!(detection.details.contains("canonical version: 0.144.4"));
        Ok(())
    }

    #[test]
    fn detect_rejects_noncanonical_version_envelopes() -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            ("legacy prefix", "codex 0.144.4\n", ""),
            ("missing terminal newline", "codex-cli 0.144.4", ""),
            ("multiple lines", "codex-cli 0.144.4\nextra\n", ""),
            ("stderr output", "codex-cli 0.144.4\n", "warning"),
        ];

        for (name, stdout, stderr) in cases {
            let dir = temp_dir("codex-detect-bad-version")?;
            let codex_home = dir.join("codex");
            let bin = dir.join("bin");
            write_fake_codex_file(&bin)?;
            let adapter = CodexAdapter::with_runner(
                CodexEnvironment {
                    home: None,
                    codex_home: Some(codex_home),
                    path: Some(bin.into_os_string()),
                },
                FakeRunner::new(vec![Ok(CommandOutput {
                    success: true,
                    status_code: Some(0),
                    stdout: stdout.to_owned(),
                    stderr: stderr.to_owned(),
                })]),
            );

            let detection = adapter.detect()?;
            assert!(!detection.available, "{name}");
            assert!(
                detection.details.contains("non-canonical"),
                "{name}: {}",
                detection.details
            );
        }
        Ok(())
    }

    #[test]
    fn verify_requires_available_executable_for_user_scope(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-verify-no-executable")?;
        let codex_home = dir.join("codex");
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: Some(dir.join("empty").into_os_string()),
        });
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&plan)?;

        let verification = adapter.verify(&plan)?;

        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Unavailable
        );
        assert!(!verification.mcp_handshake_allowed);
        assert!(verification.details.contains("install Codex"));
        Ok(())
    }

    #[test]
    fn verify_reports_failed_executable_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-verify-version-fails")?;
        let codex_home = dir.join("codex");
        let bin = dir.join("bin");
        write_fake_codex_file(&bin)?;
        let mut adapter = CodexAdapter::with_runner(
            CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: Some(bin.into_os_string()),
            },
            FakeRunner::new(vec![Ok(failed_output(42))]),
        );
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&plan)?;

        let verification = adapter.verify(&plan)?;

        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Unavailable
        );
        assert!(verification.details.contains("status 42"));
        assert!(verification
            .diagnostic
            .as_deref()
            .unwrap_or_default()
            .contains("status 42"));
        Ok(())
    }

    #[test]
    fn verify_reports_launch_failure() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-verify-launch-fails")?;
        let codex_home = dir.join("codex");
        let bin = dir.join("bin");
        write_fake_codex_file(&bin)?;
        let mut adapter = CodexAdapter::with_runner(
            CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: Some(bin.into_os_string()),
            },
            FakeRunner::new(vec![Err("permission denied".to_owned())]),
        );
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&plan)?;

        let verification = adapter.verify(&plan)?;

        assert_eq!(verification.status.as_str(), "action_required");
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Unavailable
        );
        assert!(verification.details.contains("could not be launched"));
        Ok(())
    }

    #[test]
    fn detect_and_verify_use_consistent_executable_status() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = temp_dir("codex-detect-verify-consistent")?;
        let codex_home = dir.join("codex");
        let mut adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(codex_home),
            path: Some(dir.join("empty").into_os_string()),
        });
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        adapter.apply(&plan)?;

        let detection = adapter.detect()?;
        let verification = adapter.verify(&plan)?;

        assert!(!detection.available);
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Unavailable
        );
        assert_eq!(verification.status.as_str(), "action_required");
        Ok(())
    }

    #[test]
    fn missing_executable_diagnostic_does_not_expose_path_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-diagnostic-path")?;
        let adapter = CodexAdapter::new(CodexEnvironment {
            home: None,
            codex_home: Some(dir.join("codex")),
            path: Some(OsString::from("/tmp/SECRET_PATH_TOKEN")),
        });

        let detection = adapter.detect()?;

        assert!(!detection.available);
        assert!(!detection.details.contains("SECRET_PATH_TOKEN"));
        Ok(())
    }

    #[test]
    fn verify_distinguishes_missing_changed_and_project_trust_diagnostics(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_dir("codex-verify")?;
        let codex_home = dir.join("codex");
        let bin = dir.join("bin");
        write_fake_codex_file(&bin)?;
        let mut adapter = CodexAdapter::with_runner(
            CodexEnvironment {
                home: None,
                codex_home: Some(codex_home),
                path: Some(bin.into_os_string()),
            },
            FakeRunner::new(vec![
                Ok(ok_output()),
                Ok(ok_output()),
                Ok(ok_output()),
                Ok(ok_output()),
            ]),
        );
        let plan = adapter.plan(request(HostScope::User, None, Path::new("/bin/volicord")))?;
        let missing = adapter.verify(&plan)?;
        assert_eq!(missing.status.as_str(), "missing");
        assert_eq!(
            missing.host_version.as_deref(),
            Some(REVIEWED_CODEX_HOST_VERSION)
        );
        adapter.apply(&plan)?;
        let configured = adapter.verify(&plan)?;
        assert_eq!(configured.host_state.as_str(), "configured_ready");
        assert_eq!(
            configured.host_version.as_deref(),
            Some(REVIEWED_CODEX_HOST_VERSION)
        );
        let HostTarget::File(target) = plan.target.clone() else {
            unreachable!("codex target");
        };
        fs::write(
            &target,
            fs::read_to_string(&target)?.replace("/bin/volicord", "/tmp/manual"),
        )?;
        assert_eq!(adapter.verify(&plan)?.status.as_str(), "changed");

        let repo = temp_dir("codex-project-verify")?;
        let project = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&project)?;
        let verification = adapter.verify(&project)?;
        assert_eq!(verification.status.as_str(), "complete");
        assert_eq!(
            verification.host_executable,
            HostExecutableStatus::Available
        );
        assert_eq!(
            verification
                .project_trust
                .as_ref()
                .expect("project trust diagnostic should be present")
                .status,
            ProjectTrustStatus::Missing
        );
        assert!(verification.user_actions.is_empty());
        assert!(verification.mcp_handshake_allowed);
        Ok(())
    }

    #[test]
    fn verify_treats_missing_managed_launch_markers_as_unmanaged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-missing-launch-markers")?;
        fs::create_dir_all(repo.join(".codex"))?;
        let adapter = CodexAdapter::new(CodexEnvironment::default());
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        fs::write(
            repo.join(".codex/config.toml"),
            "[mcp_servers.volicord]\ncommand = \"volicord\"\nargs = [\"mcp\", \"--stdio\", \"--connection\", \"int_alpha\", \"--project\", \"project_alpha\"]\n",
        )?;

        let status = managed_identity_evaluation_for_plan(&plan)?.status;

        assert_eq!(status, ManagedConfigStatus::Unmanaged);
        Ok(())
    }

    #[test]
    fn verify_treats_repository_discovery_env_injection_as_unmanaged(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let repo = temp_dir("codex-launch-marker-mismatch")?;
        fs::create_dir_all(repo.join(".codex"))?;
        let mut adapter = CodexAdapter::new(CodexEnvironment::default());
        let plan = adapter.plan(request(
            HostScope::Project,
            Some(&repo),
            Path::new("ignored"),
        ))?;
        adapter.apply(&plan)?;
        let target = repo.join(".codex/config.toml");
        fs::write(
            &target,
            format!(
                "{}\n[mcp_servers.volicord.env]\nSECRET_TOKEN = \"local-only\"\n",
                fs::read_to_string(&target)?
            ),
        )?;

        let status = managed_identity_evaluation_for_plan(&plan)?.status;

        assert_eq!(status, ManagedConfigStatus::Unmanaged);
        Ok(())
    }

    fn request<'a>(
        scope: HostScope,
        repo_root: Option<&'a Path>,
        mcp_command: &'a Path,
    ) -> HostPlanRequest<'a> {
        let connection_intent = match scope {
            HostScope::User => ConnectionIntent::Personal,
            HostScope::Project => ConnectionIntent::Shared,
            _ => ConnectionIntent::Personal,
        };
        HostPlanRequest {
            host_kind: HostKind::Codex,
            connection_intent,
            project: repo_root.map(|repo_root| ProjectContext {
                project_id: "project_alpha",
                project_name: "Alpha",
                repo_root,
            }),
            installation_profile: InstallationProfile {
                runtime_home: Path::new("/runtime"),
                volicord_command: Path::new("/bin/volicord"),
                volicord_mcp_command: mcp_command,
                default_connection_mode: "workflow",
            },
            connection_id: "int_alpha",
            mode: "workflow",
            expected_fingerprint: None,
        }
    }

    fn append_tool_approval_overlay(
        target: &Path,
        tool_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut text = fs::read_to_string(target)?;
        text.push_str(&format!(
            "\n[mcp_servers.volicord.tools.\"{tool_name}\"]\napproval_mode = \"approve\"\n"
        ));
        fs::write(target, text)?;
        Ok(())
    }

    fn existing_request<'a>(
        scope: HostScope,
        config_target: &'a Path,
        mcp_command: &'a Path,
        runtime_home: Option<&'a Path>,
    ) -> CodexExistingPlanRequest<'a> {
        CodexExistingPlanRequest {
            connection_intent: match scope {
                HostScope::Project => ConnectionIntent::Shared,
                _ => ConnectionIntent::Personal,
            },
            scope,
            connection_id: "int_alpha",
            project_id: (scope == HostScope::Project).then_some("project_alpha"),
            server_name: "volicord-existing",
            config_target,
            mcp_command,
            runtime_home,
            mode: "workflow",
        }
    }

    fn temp_dir(prefix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    fn write_fake_codex_file(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(dir)?;
        fs::write(dir.join("codex"), "fake codex")?;
        Ok(())
    }

    fn write_project_trust(
        codex_home: &Path,
        repo_root: &Path,
        trust_level: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(codex_home)?;
        fs::write(
            codex_home.join("config.toml"),
            format!(
                "[projects.\"{}\"]\ntrust_level = \"{}\"\n",
                repo_root.display(),
                trust_level
            ),
        )?;
        Ok(())
    }

    fn ok_output() -> CommandOutput {
        CommandOutput {
            success: true,
            status_code: Some(0),
            stdout: "codex-cli 0.144.4\n".to_owned(),
            stderr: String::new(),
        }
    }

    fn failed_output(status_code: i32) -> CommandOutput {
        CommandOutput {
            success: false,
            status_code: Some(status_code),
            stdout: String::new(),
            stderr: "version failed".to_owned(),
        }
    }

    #[derive(Debug)]
    struct FakeRunner {
        outputs: VecDeque<Result<CommandOutput, String>>,
        calls: Vec<CommandInvocation>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<Result<CommandOutput, String>>) -> Self {
            Self {
                outputs: outputs.into(),
                calls: Vec::new(),
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&mut self, invocation: &CommandInvocation) -> Result<CommandOutput, String> {
            self.calls.push(invocation.clone());
            self.outputs
                .pop_front()
                .unwrap_or_else(|| Err("missing fake command output".to_owned()))
        }
    }
}
