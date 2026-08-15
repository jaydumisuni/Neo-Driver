use crate::model::{
    canonical, DebloatPreparedStep, DebloatPreparedTransaction, DebloatRestoreRoute,
    ExactAppxInventory, ExactPackageDependency, ExactPackageIdentity,
};
use crate::DebloatPlanError;
use neo_core::{ActionKind, EvidenceItem, PlannedAction, RebootRequirement};
use neo_debloat::{
    assess_debloat, DebloatCatalogue, DebloatDisposition, DebloatEvidence, DebloatProfile,
    DebloatScope, ObservedPresence, RestoreMethod,
};
use neo_transaction::{
    CapturedState, CapturedValue, RollbackPlan, StateTarget, StateTargetKind, TransactionAction,
    TransactionCheckpoint, TransactionPlan, VerificationExpectation, VerificationPredicate,
};

pub fn prepare_debloat_transaction_from_evidence(
    catalogue: &DebloatCatalogue,
    evidence: &DebloatEvidence,
    inventory: &ExactAppxInventory,
    profile: DebloatProfile,
    selected_ids: &[String],
    mission_id: impl Into<String>,
) -> Result<DebloatPreparedTransaction, DebloatPlanError> {
    inventory.validate()?;
    if selected_ids.len() != 1 {
        return Err(DebloatPlanError::BatchNotSupported);
    }
    let mission_id = mission_id.into();
    if mission_id.trim().is_empty() {
        return Err(DebloatPlanError::InvalidRequest(
            "mission id must not be empty".to_string(),
        ));
    }

    let assessment = assess_debloat(catalogue, evidence, profile, selected_ids)?;
    let assessed = assessment
        .items
        .first()
        .ok_or(DebloatPlanError::BatchNotSupported)?;
    if assessed.disposition != DebloatDisposition::RemovalCandidate {
        return Err(DebloatPlanError::NotRemovalCandidate(assessed.id.clone()));
    }
    if assessed.scope != DebloatScope::CurrentUser {
        return Err(DebloatPlanError::UnsupportedScope(assessed.id.clone()));
    }
    if assessed.installed != ObservedPresence::Present {
        return Err(DebloatPlanError::InventoryDrift(format!(
            "{} is not present for the current user",
            assessed.package_id
        )));
    }

    let definition = catalogue
        .get(&assessed.id)
        .ok_or_else(|| DebloatPlanError::NotRemovalCandidate(assessed.id.clone()))?;
    if !matches!(definition.restore, RestoreMethod::ProvisionedImage) {
        return Err(DebloatPlanError::RestoreNotReady(format!(
            "{} requires a matching provisioned staged identity, not {:?}",
            definition.id, definition.restore
        )));
    }
    if assessed.provisioned != ObservedPresence::Present {
        return Err(DebloatPlanError::RestoreNotReady(format!(
            "{} has no proven provisioned identity",
            definition.package_id
        )));
    }

    let current_matches = inventory.current_by_name(&definition.package_id);
    let current = exact_one(
        current_matches,
        &format!("current-user {}", definition.package_id),
    )?;
    ensure_removal_kind(current)?;

    let provisioned_matches = inventory.provisioned_by_name(&definition.package_id);
    let provisioned = exact_one(
        provisioned_matches,
        &format!("provisioned {}", definition.package_id),
    )?;
    ensure_removal_kind(provisioned)?;
    if canonical(&current.full_name) != canonical(&provisioned.full_name)
        || canonical(&current.family_name) != canonical(&provisioned.family_name)
    {
        return Err(DebloatPlanError::InventoryDrift(format!(
            "current/provisioned exact identity mismatch for {}",
            definition.package_id
        )));
    }

    for dependency in &current.dependencies {
        ensure_dependency_restore_ready(inventory, dependency)?;
    }

    let main_target = appx_target(&current.full_name);
    let mut snapshot_targets = vec![main_target.clone()];
    let mut baseline_states = vec![CapturedState {
        target: main_target.clone(),
        value: CapturedValue::Present(serde_json::to_string(current)?),
    }];
    let mut rollback_verification = vec![VerificationPredicate {
        id: format!("rollback:{}:main", definition.id),
        target: main_target.clone(),
        expectation: VerificationExpectation::MatchesBaseline,
        required: true,
    }];

    let mut dependency_full_names = Vec::with_capacity(current.dependencies.len());
    for (index, dependency) in current.dependencies.iter().enumerate() {
        let target = appx_target(&dependency.full_name);
        snapshot_targets.push(target.clone());
        baseline_states.push(CapturedState {
            target: target.clone(),
            value: CapturedValue::Present(serde_json::to_string(dependency)?),
        });
        rollback_verification.push(VerificationPredicate {
            id: format!("rollback:{}:dependency:{index}", definition.id),
            target,
            expectation: VerificationExpectation::MatchesBaseline,
            required: true,
        });
        dependency_full_names.push(dependency.full_name.clone());
    }

    let action = TransactionAction {
        action: PlannedAction {
            id: definition.id.clone(),
            title: format!("Remove {} for current user", definition.title),
            kind: ActionKind::Debloat,
            risk: definition.risk,
            recommendation: definition.recommendation,
            verdict: definition.verdict,
            rationale: "Prepare controlled current-user AppX removal only after exact package/dependency identity capture and deterministic provisioned-image re-registration readiness proof.".to_string(),
            selected_by_default: false,
            requires_confirmation: true,
            requires_admin: false,
            reboot: RebootRequirement::None,
            rollback_available: true,
            evidence: vec![
                EvidenceItem::new(
                    "package_id",
                    definition.package_id.clone(),
                    "Phase 13 certified debloat definition",
                ),
                EvidenceItem::new(
                    "package_full_name",
                    current.full_name.clone(),
                    "Phase 15 native PackageManager current-user inventory",
                ),
                EvidenceItem::new(
                    "package_family_name",
                    current.family_name.clone(),
                    "Phase 15 native PackageManager identity",
                ),
                EvidenceItem::new(
                    "provisioned_restore_identity",
                    provisioned.full_name.clone(),
                    "Phase 15 native PackageManager provisioned inventory",
                ),
                EvidenceItem::new(
                    "dependency_count",
                    current.dependencies.len().to_string(),
                    "Phase 15 native Package.Dependencies inventory",
                ),
            ],
            warnings: vec![
                "Phase 15 prepares evidence and transaction state only; no AppX mutation capability is issued.".to_string(),
                "Windows may remove unneeded dependency registrations with the main package; every direct dependency is captured as a rollback target and must have a matching provisioned staged identity.".to_string(),
            ],
        },
        snapshot_targets: snapshot_targets.clone(),
        postconditions: vec![VerificationPredicate {
            id: format!("verify:{}:main-absent", definition.id),
            target: main_target,
            expectation: VerificationExpectation::Absent,
            required: true,
        }],
        rollback: RollbackPlan::Reversible {
            restore_targets: snapshot_targets,
            verification: rollback_verification,
        },
    };

    let transaction = TransactionPlan::new(
        format!("{mission_id}:phase15-debloat-current-user"),
        1,
        mission_id,
        vec![action],
    )?;
    let mut checkpoint = TransactionCheckpoint::new(transaction.clone())?;
    checkpoint.capture_baseline(baseline_states)?;

    let step = DebloatPreparedStep {
        debloat_id: definition.id.clone(),
        package_id: definition.package_id.clone(),
        package_full_name: current.full_name.clone(),
        package_family_name: current.family_name.clone(),
        dependency_full_names: dependency_full_names.clone(),
        restore: DebloatRestoreRoute::RegisterByFullNameFromProvisioned {
            package_full_name: current.full_name.clone(),
            package_family_name: current.family_name.clone(),
            dependency_full_names,
        },
    };

    Ok(DebloatPreparedTransaction {
        assessment,
        steps: vec![step],
        transaction,
        checkpoint,
        machine_changes: false,
    })
}

fn exact_one<'a>(
    matches: Vec<&'a ExactPackageIdentity>,
    label: &str,
) -> Result<&'a ExactPackageIdentity, DebloatPlanError> {
    match matches.as_slice() {
        [] => Err(DebloatPlanError::MissingExactIdentity(label.to_string())),
        [package] => Ok(*package),
        _ => Err(DebloatPlanError::AmbiguousExactIdentity(label.to_string())),
    }
}

fn ensure_removal_kind(package: &ExactPackageIdentity) -> Result<(), DebloatPlanError> {
    if package.is_framework || package.is_resource {
        return Err(DebloatPlanError::UnsafePackageKind(
            package.full_name.clone(),
        ));
    }
    Ok(())
}

fn ensure_dependency_restore_ready(
    inventory: &ExactAppxInventory,
    dependency: &ExactPackageDependency,
) -> Result<(), DebloatPlanError> {
    let matches = inventory.provisioned_exact(&dependency.full_name, &dependency.family_name);
    match matches.as_slice() {
        [_] => Ok(()),
        [] => Err(DebloatPlanError::RestoreNotReady(format!(
            "dependency {} is not present as the exact provisioned staged identity",
            dependency.full_name
        ))),
        _ => Err(DebloatPlanError::AmbiguousExactIdentity(format!(
            "dependency {} has multiple provisioned exact identities",
            dependency.full_name
        ))),
    }
}

fn appx_target(full_name: &str) -> StateTarget {
    StateTarget {
        kind: StateTargetKind::AppxPackage,
        key: format!("current_user:{full_name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExactPackageDependency, ExactPackageIdentity};
    use neo_transaction::TransactionStage;

    fn catalogue(scope: &str, restore: &str) -> DebloatCatalogue {
        serde_json::from_str(&format!(
            r#"{{"items":[{{"id":"appx.contoso.phase15","package_id":"Contoso.Phase15","title":"Contoso Phase15","category":"Fixture","description":"Synthetic Phase 15 package","class":"safe_optional","scope":"{scope}","risk":"low","recommendation":"optional_component","verdict":"certified","selected_by_default":false,"restore":{restore},"side_effects":[],"preserve_in_profiles":[]}}]}}"#
        ))
        .expect("catalogue must validate")
    }

    fn evidence(installed: &str, provisioned: &str) -> DebloatEvidence {
        serde_json::from_str(&format!(
            r#"{{"observations":[{{"package_id":"Contoso.Phase15","installed":"{installed}","provisioned":"{provisioned}","version":"1.2.3.4","source":"phase15-test"}}]}}"#
        ))
        .expect("evidence must validate")
    }

    fn identity() -> ExactPackageIdentity {
        ExactPackageIdentity {
            name: "Contoso.Phase15".to_string(),
            full_name: "Contoso.Phase15_1.2.3.4_x64__contoso".to_string(),
            family_name: "Contoso.Phase15_contoso".to_string(),
            is_framework: false,
            is_resource: false,
            is_bundle: false,
            is_optional: false,
            dependencies: vec![ExactPackageDependency {
                name: "Contoso.Framework".to_string(),
                full_name: "Contoso.Framework_1.0.0.0_x64__contoso".to_string(),
                family_name: "Contoso.Framework_contoso".to_string(),
            }],
        }
    }

    fn inventory() -> ExactAppxInventory {
        let current = identity();
        let dependency = ExactPackageIdentity {
            name: "Contoso.Framework".to_string(),
            full_name: current.dependencies[0].full_name.clone(),
            family_name: current.dependencies[0].family_name.clone(),
            is_framework: true,
            is_resource: false,
            is_bundle: false,
            is_optional: false,
            dependencies: Vec::new(),
        };
        ExactAppxInventory::new(
            vec![current.clone(), dependency.clone()],
            vec![current, dependency],
            "phase15-test-native",
        )
        .expect("inventory must validate")
    }

    #[test]
    fn prepares_exact_current_user_transaction_without_mutation_authority() {
        let prepared = prepare_debloat_transaction_from_evidence(
            &catalogue("current_user", r#"{"kind":"provisioned_image"}"#),
            &evidence("present", "present"),
            &inventory(),
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect("transaction readiness should be proven");

        assert!(!prepared.machine_changes);
        assert_eq!(prepared.steps.len(), 1);
        assert_eq!(prepared.transaction.actions().len(), 1);
        assert_eq!(
            prepared.checkpoint.stage(),
            TransactionStage::BaselineCaptured
        );
        assert_eq!(
            prepared.checkpoint.plan_fingerprint(),
            prepared
                .transaction
                .fingerprint()
                .expect("fingerprint must compute")
        );
        assert_eq!(
            prepared.transaction.actions()[0].action.kind,
            ActionKind::Debloat
        );
        assert_eq!(prepared.transaction.actions()[0].snapshot_targets.len(), 2);
    }

    #[test]
    fn store_metadata_is_not_treated_as_executable_rollback() {
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue(
                "current_user",
                r#"{"kind":"store","store_id":"9CONTOSO15"}"#,
            ),
            &evidence("present", "present"),
            &inventory(),
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect_err("Store metadata is not deterministic local rollback authority");
        assert!(matches!(error, DebloatPlanError::RestoreNotReady(_)));
    }

    #[test]
    fn combined_or_provisioned_scope_stays_blocked() {
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue(
                "current_user_and_provisioned",
                r#"{"kind":"provisioned_image"}"#,
            ),
            &evidence("present", "present"),
            &inventory(),
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect_err("provisioning mutation remains separately gated");
        assert!(matches!(error, DebloatPlanError::UnsupportedScope(_)));
    }

    #[test]
    fn exact_identity_drift_fails_closed() {
        let mut drifted = inventory();
        drifted.current_user[0].full_name = "Contoso.Phase15_9.9.9.9_x64__contoso".to_string();
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue("current_user", r#"{"kind":"provisioned_image"}"#),
            &evidence("present", "present"),
            &drifted,
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect_err("drift must block authority preparation");
        assert!(matches!(error, DebloatPlanError::InventoryDrift(_)));
    }

    #[test]
    fn missing_dependency_restore_identity_fails_closed() {
        let mut missing = inventory();
        missing
            .provisioned
            .retain(|package| package.name != "Contoso.Framework");
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue("current_user", r#"{"kind":"provisioned_image"}"#),
            &evidence("present", "present"),
            &missing,
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect_err("dependency restore gap must block planning");
        assert!(matches!(error, DebloatPlanError::RestoreNotReady(_)));
    }

    #[test]
    fn batch_preparation_is_deliberately_blocked() {
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue("current_user", r#"{"kind":"provisioned_image"}"#),
            &evidence("present", "present"),
            &inventory(),
            DebloatProfile::SafeCleanup,
            &[
                "appx.contoso.phase15".to_string(),
                "appx.contoso.phase15.other".to_string(),
            ],
            "mission-phase15",
        )
        .expect_err("Phase 15 is single-item only");
        assert!(matches!(error, DebloatPlanError::BatchNotSupported));
    }
}
