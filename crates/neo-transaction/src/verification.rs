use crate::error::TransactionError;
use crate::plan::{ApplyRecord, RollbackRecord, TransactionPlan};
use crate::state::{
    BaselineSnapshot, Observation, ObservedValue, VerificationPredicate, VerificationResult,
    VerificationStatus,
};
use std::collections::BTreeSet;

pub(crate) fn evaluate_predicates(
    predicates: &[VerificationPredicate],
    observations: &[Observation],
) -> Result<Vec<VerificationResult>, TransactionError> {
    let expected_targets = predicates
        .iter()
        .map(|predicate| predicate.target.clone())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for observation in observations {
        observation.target.validate()?;
        if !expected_targets.contains(&observation.target) {
            return Err(TransactionError::UnexpectedObservation(
                observation.target.key.clone(),
            ));
        }
        if !seen.insert(observation.target.clone()) {
            return Err(TransactionError::DuplicateObservation(
                observation.target.key.clone(),
            ));
        }
    }

    let results = predicates
        .iter()
        .map(|predicate| {
            let observed = observations
                .iter()
                .find(|observation| observation.target == predicate.target)
                .map(|observation| observation.value.clone())
                .unwrap_or_else(|| {
                    ObservedValue::Unavailable("required observation missing".to_string())
                });
            VerificationResult {
                predicate: predicate.clone(),
                observed,
            }
        })
        .collect::<Vec<_>>();

    Ok(results)
}

pub(crate) fn required_results_pass(
    results: &[VerificationResult],
    baseline: &BaselineSnapshot,
) -> bool {
    results.iter().all(|result| {
        !result.predicate.required || result.status(baseline) == VerificationStatus::Pass
    })
}

pub(crate) fn validate_result_set(
    results: &[VerificationResult],
    predicates: &[VerificationPredicate],
    baseline: &BaselineSnapshot,
    require_pass: bool,
) -> Result<(), TransactionError> {
    if results.len() != predicates.len() {
        return Err(TransactionError::VerificationCoverageMismatch);
    }
    for predicate in predicates {
        let result = results
            .iter()
            .find(|result| result.predicate.id == predicate.id)
            .ok_or(TransactionError::VerificationCoverageMismatch)?;
        if &result.predicate != predicate {
            return Err(TransactionError::VerificationPredicateMismatch(
                predicate.id.clone(),
            ));
        }
    }
    if require_pass && !required_results_pass(results, baseline) {
        return Err(TransactionError::RequiredVerificationNotProven);
    }
    if !require_pass && required_results_pass(results, baseline) {
        return Err(TransactionError::ExpectedUnprovenVerification);
    }
    Ok(())
}

pub(crate) fn validate_record_ids(
    records: &[ApplyRecord],
    plan: &TransactionPlan,
) -> Result<(), TransactionError> {
    let mut seen = BTreeSet::new();
    for record in records {
        if plan.action_by_id(&record.action_id).is_none() {
            return Err(TransactionError::UnknownApplyAction(record.action_id.clone()));
        }
        if !seen.insert(record.action_id.as_str()) {
            return Err(TransactionError::DuplicateApplyRecord(
                record.action_id.clone(),
            ));
        }
        if record.detail.trim().is_empty() {
            return Err(TransactionError::EmptyExecutionDetail(
                record.action_id.clone(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_rollback_record_ids(
    records: &[RollbackRecord],
    plan: &TransactionPlan,
) -> Result<(), TransactionError> {
    let mut seen = BTreeSet::new();
    for record in records {
        if plan.action_by_id(&record.action_id).is_none() {
            return Err(TransactionError::UnknownRollbackAction(
                record.action_id.clone(),
            ));
        }
        if !seen.insert(record.action_id.as_str()) {
            return Err(TransactionError::DuplicateRollbackRecord(
                record.action_id.clone(),
            ));
        }
        if record.detail.trim().is_empty() {
            return Err(TransactionError::EmptyExecutionDetail(
                record.action_id.clone(),
            ));
        }
    }
    Ok(())
}
