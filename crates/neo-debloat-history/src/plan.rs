use crate::model::{appx_target, DebloatRemovalReceipt, DebloatRestorePreparedTransaction};
use crate::DebloatHistoryError;
use neo_core::{
    ActionKind, EvidenceItem, EvidenceVerdict, PlannedAction, RebootRequirement,
    RecommendationState,
};
use neo_debloat_plan::{ExactAppxInventory, ExactPackageDependency, ExactPackageIdentity};
use neo_transaction::{
    CapturedState, CapturedValue, RollbackPlan, TransactionAction, TransactionCheckpoint,
    TransactionPlan, VerificationExpectation, VerificationPredicate,
};

pub fn prepare_restore_from_inventory(
    receipt: &DebloatRemovalReceipt,
    inventory: &ExactAppxInventory,
    mission_id: impl Into<String>,
) -> Result<DebloatRestorePreparedTransaction, DebloatHistoryError> {
    receipt.validate()?;
    inventory.validate()?;
    let mission_id = mission_id.into();
    if mission_id.trim().is_empty() {
        return Err(DebloatHistoryError::RestoreNotReady(
            "restore mission id must not be empty".to_string(),
        ));
    }

    ensure_main_restore_state(receipt, inventory)?;
    ensure_provisioned_restore_route(receipt, inventory)?;
    for dependency in receipt.dependencies() {
        ensure_dependency_restore_state(dependency, inventory)?;
    }

    let main_target = appx_target(&receipt.main().full_name);
    let mut snapshot_targets = vec![main_target.clone()];
    let mut baseline_states = vec![CapturedState {
        target: main_target.clone(),
        value: CapturedValue::Absent,
    }];
    let mut postconditions = vec![VerificationPredicate {
        id: format!("verify:restore:{}:main", receipt.debloat_id()),
        target: main_target.clone(),
        expectation: VerificationExpectation::Equals(serde_json::to_string(receipt.main())?),
        required: true,
    }];
    let mut rollback_verification = vec![VerificationPredicate {
        id: format!("rollback:restore:{}:main", receipt.debloat_id()),
        target: main_target,
        expectation: VerificationExpectation::MatchesBaseline,
        required: true,
    }];

    for (index, dependency) in receipt.dependencies().iter().enumerate() {
        let target = appx_target(&dependency.full_name);
        snapshot_targets.push(target.clone());
        baseline_states.push(CapturedState {
            target: target.clone(),
            value: current_dependency_baseline(dependency, inventory)?,
        });
        postconditions.push(VerificationPredicate {
            id: format!("verify:restore:{}:dependency:{index}", receipt.debloat_id()),
            target: target.clone(),
            expectation: VerificationExpectation::Equals(serde_json::to_string(dependency)?),
            required: true,
        });
        rollback_verification.push(VerificationPredicate {
            id: format!(
                "rollback:restore:{}:dependency:{index}",
                receipt.debloat_id()
            ),
            target,
            expectation: VerificationExpectation::MatchesBaseline,
            required: true,
        });
    }

    let source_action = &receipt.source_checkpoint().plan().actions()[0].action;
    let restore_action_id = format!("restore:{}", receipt.debloat_id());
    let action = TransactionAction {
        action: PlannedAction {
            id: restore_action_id,
            title: format!("Restore {} for current user", receipt.package_id()),
            kind: ActionKind::Debloat,
            risk: source_action.risk,
            recommendation: RecommendationState::Repair,
            verdict: EvidenceVerdict::Certified,
            rationale: "Prepare an explicit post-success restore only from a validated completed-removal receipt after re-proving the exact local staged package and direct-dependency identities.".to_string(),
            selected_by_default: false,
            requires_confirmation: true,
            requires_admin: false,
            reboot: RebootRequirement::None,
            rollback_available: true,
            evidence: vec![
                EvidenceItem::new(
                    "phase17_receipt_fingerprint",
                    receipt.receipt_fingerprint().to_string(),
                    "Phase 17 validated completed-removal history receipt",
                ),
                EvidenceItem::new(
                    "source_transaction_fingerprint",
                    receipt.source_transaction_fingerprint().to_string(),
                    "Phase 16 completed transaction checkpoint",
                ),
                EvidenceItem::new(
                    "restore_package_full_name",
                    receipt.main().full_name.clone(),
                    "Phase 17 current native provisioned AppX inventory",
                ),
                EvidenceItem::new(
                    "restore_dependency_count",
                    receipt.dependencies().len().to_string(),
                    "Phase 17 exact staged dependency re-proof",
                ),
            ],
            warnings: vec![
                "Phase 17 prepares history and inverse transaction state only; it does not issue post-success restore mutation authority.".to_string(),
                "A future restore executor must restore the original package/dependency identities and, on failure, return to this restore-time baseline rather than assuming the original pre-removal baseline.".to_string(),
            ],
        },
        snapshot_targets: snapshot_targets.clone(),
        postconditions,
        rollback: RollbackPlan::Reversible {
            restore_targets: snapshot_targets,
            verification: rollback_verification,
        },
    };

    let transaction = TransactionPlan::new(
        format!("{mission_id}:phase17-debloat-restore-current-user"),
        1,
        mission_id,
        vec![action],
    )?;
    let mut checkpoint = TransactionCheckpoint::new(transaction.clone())?;
    checkpoint.capture_baseline(baseline_states)?;

    Ok(DebloatRestorePreparedTransaction::new(
        receipt,
        transaction,
        checkpoint,
    ))
}

fn ensure_main_restore_state(
    receipt: &DebloatRemovalReceipt,
    inventory: &ExactAppxInventory,
) -> Result<(), DebloatHistoryError> {
    for current in &inventory.current_user {
        if current
            .full_name
            .eq_ignore_ascii_case(&receipt.main().full_name)
        {
            if same_main_restore_shape(current, receipt.main()) {
                return Err(DebloatHistoryError::AlreadyRestored);
            }
            return Err(DebloatHistoryError::InventoryConflict(format!(
                "exact main full name {} is registered with a different identity shape",
                receipt.main().full_name
            )));
        }
        if current.name.eq_ignore_ascii_case(&receipt.main().name)
            || current
                .family_name
                .eq_ignore_ascii_case(&receipt.main().family_name)
        {
            return Err(DebloatHistoryError::InventoryConflict(format!(
                "a different current-user version/identity of {} is already registered: {}",
                receipt.main().name,
                current.full_name
            )));
        }
    }
    Ok(())
}

fn ensure_provisioned_restore_route(
    receipt: &DebloatRemovalReceipt,
    inventory: &ExactAppxInventory,
) -> Result<(), DebloatHistoryError> {
    let matches = inventory
        .provisioned
        .iter()
        .filter(|package| {
            package
                .full_name
                .eq_ignore_ascii_case(receipt.restore().package_full_name())
                && package
                    .family_name
                    .eq_ignore_ascii_case(receipt.restore().package_family_name())
        })
        .collect::<Vec<_>>();
    let main = match matches.as_slice() {
        [package] => *package,
        [] => {
            return Err(DebloatHistoryError::RestoreNotReady(format!(
                "exact staged main identity {} is no longer provisioned",
                receipt.restore().package_full_name()
            )))
        }
        _ => {
            return Err(DebloatHistoryError::InventoryConflict(format!(
                "multiple staged exact identities match {}",
                receipt.restore().package_full_name()
            )))
        }
    };
    if !same_main_restore_shape(main, receipt.main()) {
        return Err(DebloatHistoryError::RestoreNotReady(format!(
            "staged main identity {} no longer matches the completed-removal receipt",
            main.full_name
        )));
    }

    for dependency in receipt.dependencies() {
        let matches = inventory
            .provisioned
            .iter()
            .filter(|package| {
                package
                    .full_name
                    .eq_ignore_ascii_case(&dependency.full_name)
                    && package
                        .family_name
                        .eq_ignore_ascii_case(&dependency.family_name)
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [package] if package.name.eq_ignore_ascii_case(&dependency.name) => {}
            [package] => {
                return Err(DebloatHistoryError::RestoreNotReady(format!(
                    "staged dependency {} has mismatched package name {}",
                    dependency.full_name, package.name
                )))
            }
            [] => {
                return Err(DebloatHistoryError::RestoreNotReady(format!(
                    "exact staged dependency {} is no longer provisioned",
                    dependency.full_name
                )))
            }
            _ => {
                return Err(DebloatHistoryError::InventoryConflict(format!(
                    "multiple staged exact identities match dependency {}",
                    dependency.full_name
                )))
            }
        }
    }
    Ok(())
}

fn ensure_dependency_restore_state(
    dependency: &ExactPackageDependency,
    inventory: &ExactAppxInventory,
) -> Result<(), DebloatHistoryError> {
    for current in &inventory.current_user {
        if current
            .full_name
            .eq_ignore_ascii_case(&dependency.full_name)
        {
            if current.name.eq_ignore_ascii_case(&dependency.name)
                && current
                    .family_name
                    .eq_ignore_ascii_case(&dependency.family_name)
            {
                return Ok(());
            }
            return Err(DebloatHistoryError::InventoryConflict(format!(
                "exact dependency full name {} has a different current-user identity",
                dependency.full_name
            )));
        }
        if current.name.eq_ignore_ascii_case(&dependency.name)
            || current
                .family_name
                .eq_ignore_ascii_case(&dependency.family_name)
        {
            return Err(DebloatHistoryError::InventoryConflict(format!(
                "a different current-user dependency version conflicts with {}: {}",
                dependency.full_name, current.full_name
            )));
        }
    }
    Ok(())
}

fn current_dependency_baseline(
    dependency: &ExactPackageDependency,
    inventory: &ExactAppxInventory,
) -> Result<CapturedValue, DebloatHistoryError> {
    let current = inventory.current_user.iter().find(|package| {
        package
            .full_name
            .eq_ignore_ascii_case(&dependency.full_name)
    });
    match current {
        Some(package)
            if package.name.eq_ignore_ascii_case(&dependency.name)
                && package
                    .family_name
                    .eq_ignore_ascii_case(&dependency.family_name) =>
        {
            Ok(CapturedValue::Present(serde_json::to_string(dependency)?))
        }
        Some(package) => Err(DebloatHistoryError::InventoryConflict(format!(
            "dependency {} baseline identity conflicts with {}",
            dependency.full_name, package.full_name
        ))),
        None => Ok(CapturedValue::Absent),
    }
}

fn same_main_restore_shape(left: &ExactPackageIdentity, right: &ExactPackageIdentity) -> bool {
    left.name.eq_ignore_ascii_case(&right.name)
        && left.full_name.eq_ignore_ascii_case(&right.full_name)
        && left.family_name.eq_ignore_ascii_case(&right.family_name)
        && left.is_framework == right.is_framework
        && left.is_resource == right.is_resource
        && left.is_bundle == right.is_bundle
        && left.is_optional == right.is_optional
        && left.dependencies.len() == right.dependencies.len()
        && left
            .dependencies
            .iter()
            .zip(&right.dependencies)
            .all(|(left, right)| {
                left.name.eq_ignore_ascii_case(&right.name)
                    && left.full_name.eq_ignore_ascii_case(&right.full_name)
                    && left.family_name.eq_ignore_ascii_case(&right.family_name)
            })
}
