use neo_transaction::{
    ApplyOutcome, ApplyRecord, Observation, ObservedValue, RollbackRecord,
    TransactionAuthorization, TransactionCheckpoint, TransactionStage,
};
use serde::{Deserialize, Serialize};

use crate::model::sha256_file;
use crate::plan::{
    baseline_contract, binding_target, normalized_id_set, policy_target, signature_matches,
    store_target, transaction_contract,
};
use crate::{
    DriverBindingBaseline, DriverHost, DriverInstallPlan, DriverInventory, DriverStoreBaseline,
    DriverStoreError, PreparedDriverInstall, StoredDriverPackage,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DriverInstallSessionWire")]
pub struct DriverInstallSession {
    driver_plan: DriverInstallPlan,
    transaction: TransactionCheckpoint,
    target_package: Option<StoredDriverPackage>,
}

#[derive(Debug, Deserialize)]
struct DriverInstallSessionWire {
    driver_plan: DriverInstallPlan,
    transaction: TransactionCheckpoint,
    target_package: Option<StoredDriverPackage>,
}

impl TryFrom<DriverInstallSessionWire> for DriverInstallSession {
    type Error = DriverStoreError;

    fn try_from(value: DriverInstallSessionWire) -> Result<Self, Self::Error> {
        let session = Self {
            driver_plan: value.driver_plan,
            transaction: value.transaction,
            target_package: value.target_package,
        };
        session.validate()?;
        Ok(session)
    }
}

impl DriverInstallSession {
    pub fn new(prepared: PreparedDriverInstall) -> Result<Self, DriverStoreError> {
        prepared.driver_plan.validate()?;
        let expected_transaction = transaction_contract(&prepared.driver_plan)?;
        if prepared.transaction_plan != expected_transaction {
            return Err(DriverStoreError::SessionInvariantViolation);
        }
        let expected_baseline =
            baseline_contract(&prepared.driver_plan, &prepared.transaction_plan)?;
        if prepared.baseline != expected_baseline {
            return Err(DriverStoreError::SessionInvariantViolation);
        }
        let target_package = match &prepared.driver_plan.store_baseline {
            DriverStoreBaseline::Existing { package } => Some(package.clone()),
            DriverStoreBaseline::Absent => None,
        };
        let mut transaction = TransactionCheckpoint::new(prepared.transaction_plan)?;
        transaction.capture_baseline(prepared.baseline.states)?;
        let session = Self {
            driver_plan: prepared.driver_plan,
            transaction,
            target_package,
        };
        session.validate()?;
        Ok(session)
    }

    pub fn from_json_str(input: &str) -> Result<Self, DriverStoreError> {
        let wire: DriverInstallSessionWire = serde_json::from_str(input)?;
        Self::try_from(wire)
    }

    pub fn driver_plan(&self) -> &DriverInstallPlan {
        &self.driver_plan
    }

    pub fn transaction(&self) -> &TransactionCheckpoint {
        &self.transaction
    }

    pub fn target_package(&self) -> Option<&StoredDriverPackage> {
        self.target_package.as_ref()
    }

    pub fn authorize(
        &mut self,
        authorization: TransactionAuthorization,
    ) -> Result<(), DriverStoreError> {
        self.validate()?;
        self.transaction.authorize(authorization)?;
        self.validate()
    }

    pub fn apply<H: DriverHost>(&mut self, host: &H) -> Result<(), DriverStoreError> {
        self.validate()?;
        if self.transaction.stage() != TransactionStage::Authorized {
            return Err(DriverStoreError::SessionInvariantViolation);
        }
        let before = self.preflight(host)?;
        self.transaction.begin_apply()?;

        let mut operational_error: Option<String> = None;
        if self.target_package.is_none() {
            match host.stage_driver(&self.driver_plan.source_inf) {
                Ok(package) => {
                    self.target_package = Some(package);
                }
                Err(error) => {
                    operational_error = Some(format!("driver staging failed: {error}"));
                    if let Ok(Some(package)) = host.find_equivalent_package(
                        &self.driver_plan.source_inf,
                        std::slice::from_ref(&self.driver_plan.expected_signature.catalog_file),
                    ) {
                        self.target_package = Some(package);
                    }
                }
            }
        }

        if operational_error.is_none() {
            if let Some(package) = self.target_package.as_ref() {
                if let Err(error) = self.validate_target_package(host, package) {
                    operational_error =
                        Some(format!("staged package verification failed: {error}"));
                }
            } else {
                operational_error =
                    Some("staging produced no recoverable package identity".to_string());
            }
        }

        let mut reboot_required = false;
        if operational_error.is_none() {
            for impact in &self.driver_plan.impacts {
                match host.install_best_match(&impact.instance_id) {
                    Ok(result) => reboot_required |= result.reboot_required,
                    Err(error) => {
                        operational_error = Some(format!(
                            "Windows per-device best-match install failed for {}: {error}",
                            impact.instance_id
                        ));
                        break;
                    }
                }
            }
        }

        let mut after = host.inventory()?;
        after.validate()?;
        let (mut policy_satisfied, unexpected) = self.evaluate_forward(&before, &after)?;
        if let Some(instance_id) = unexpected {
            policy_satisfied = false;
            operational_error.get_or_insert_with(|| {
                format!("unexpected binding change outside authority: {instance_id}")
            });
        }

        let binding_changed = self.impacted_binding_changed(&after);
        if !binding_changed
            && matches!(self.driver_plan.store_baseline, DriverStoreBaseline::Absent)
        {
            if let Some(package) = self.target_package.as_ref() {
                if !package_in_use(&after, &package.published_inf) {
                    if let Err(error) = host.remove_published_package(&package.published_inf) {
                        operational_error.get_or_insert_with(|| {
                            format!("unused staged package cleanup failed: {error}")
                        });
                    } else {
                        after = host.inventory()?;
                        after.validate()?;
                    }
                }
            }
        }

        let store_changed = !self.store_matches_baseline(host)?;
        let machine_changed = binding_changed || store_changed;
        if operational_error.is_none() && !policy_satisfied {
            operational_error = Some(DriverStoreError::PolicyUnsatisfied.to_string());
        }
        let outcome = if operational_error.is_none() {
            ApplyOutcome::Success
        } else {
            ApplyOutcome::Failure
        };
        let detail = operational_error.unwrap_or_else(|| {
            if machine_changed {
                "Windows best-match policy satisfied with authorized machine changes".to_string()
            } else {
                "Windows best-match policy satisfied; current healthy binding remained preferred"
                    .to_string()
            }
        });
        self.transaction.record_apply_result(ApplyRecord {
            action_id: self.driver_plan.action_id.clone(),
            outcome,
            detail,
            machine_changed,
            reboot_required,
        })?;

        if outcome == ApplyOutcome::Success
            && self.transaction.stage() == TransactionStage::Verifying
        {
            let observation = self.policy_observation(host)?;
            self.transaction.verify_postconditions(vec![observation])?;
        }
        self.validate()
    }

    pub fn resume_after_reboot<H: DriverHost>(&mut self, host: &H) -> Result<(), DriverStoreError> {
        self.validate()?;
        let observation = self.policy_observation(host)?;
        self.transaction
            .resume_after_reboot(vec![observation.clone()])?;
        if self.transaction.stage() == TransactionStage::Verifying {
            self.transaction.verify_postconditions(vec![observation])?;
        }
        self.validate()
    }

    pub fn reprobe_after_block<H: DriverHost>(&mut self, host: &H) -> Result<(), DriverStoreError> {
        self.validate()?;
        let observation = self.policy_observation(host)?;
        self.transaction
            .reprobe_after_block(vec![observation.clone()])?;
        if self.transaction.stage() == TransactionStage::Verifying {
            self.transaction.verify_postconditions(vec![observation])?;
        }
        self.validate()
    }

    pub fn rollback<H: DriverHost>(&mut self, host: &H) -> Result<(), DriverStoreError> {
        self.validate()?;
        if self.transaction.stage() != TransactionStage::RollingBack {
            return Err(DriverStoreError::SessionInvariantViolation);
        }
        let mut reboot_required = false;
        let inventory = host.inventory()?;
        inventory.validate()?;
        for impact in &self.driver_plan.impacts {
            let current = inventory.device(&impact.instance_id);
            if current_binding(current) == Some(impact.baseline.clone()) {
                continue;
            }
            let baseline_inf = impact.baseline_package.published_inf.as_str();
            match host.restore_specific_driver(&impact.instance_id, baseline_inf) {
                Ok(result) => reboot_required |= result.reboot_required,
                Err(error) => {
                    self.transaction.record_rollback_result(RollbackRecord {
                        action_id: self.driver_plan.action_id.clone(),
                        outcome: ApplyOutcome::Failure,
                        detail: format!(
                            "captured binding restore failed for {}: {error}",
                            impact.instance_id
                        ),
                        reboot_required,
                    })?;
                    return self.validate();
                }
            }
        }

        if !reboot_required {
            if let Err(error) = self.restore_driver_store_if_possible(host) {
                self.transaction.record_rollback_result(RollbackRecord {
                    action_id: self.driver_plan.action_id.clone(),
                    outcome: ApplyOutcome::Failure,
                    detail: format!("Driver Store restoration failed: {error}"),
                    reboot_required: false,
                })?;
                return self.validate();
            }
        }

        self.transaction.record_rollback_result(RollbackRecord {
            action_id: self.driver_plan.action_id.clone(),
            outcome: ApplyOutcome::Success,
            detail: if reboot_required {
                "captured bindings restored; reboot required before Driver Store restoration proof"
                    .to_string()
            } else {
                "captured bindings and Driver Store state restored".to_string()
            },
            reboot_required,
        })?;

        if self.transaction.stage() == TransactionStage::RollingBack {
            let observations = self.rollback_observations(host)?;
            self.transaction.verify_rollback(observations)?;
        }
        self.validate()
    }

    pub fn resume_after_rollback_reboot<H: DriverHost>(
        &mut self,
        host: &H,
    ) -> Result<(), DriverStoreError> {
        self.validate()?;
        if self.transaction.stage() != TransactionStage::AwaitingRollbackReboot {
            return Err(DriverStoreError::SessionInvariantViolation);
        }
        let _ = self.restore_driver_store_if_possible(host);
        let observations = self.rollback_observations(host)?;
        self.transaction
            .resume_after_rollback_reboot(observations)?;
        self.validate()
    }

    pub fn validate(&self) -> Result<(), DriverStoreError> {
        self.driver_plan.validate()?;
        self.transaction.validate()?;
        let expected_transaction = transaction_contract(&self.driver_plan)?;
        if self.transaction.plan() != &expected_transaction {
            return Err(DriverStoreError::SessionInvariantViolation);
        }
        let expected_baseline = baseline_contract(&self.driver_plan, &expected_transaction)?;
        if self.transaction.baseline() != Some(&expected_baseline) {
            return Err(DriverStoreError::SessionInvariantViolation);
        }
        if let DriverStoreBaseline::Existing { package } = &self.driver_plan.store_baseline {
            if self.target_package.as_ref() != Some(package) {
                return Err(DriverStoreError::SessionInvariantViolation);
            }
        }
        if let Some(package) = &self.target_package {
            package.validate()?;
        }
        Ok(())
    }

    fn preflight<H: DriverHost>(&self, host: &H) -> Result<DriverInventory, DriverStoreError> {
        if sha256_file(&self.driver_plan.source_inf)? != self.driver_plan.source_inf_sha256 {
            return Err(DriverStoreError::PrestateDrift);
        }
        let signature = host.verify_inf_signature(&self.driver_plan.source_inf)?;
        if !signature_matches(&signature, &self.driver_plan.expected_signature) {
            return Err(DriverStoreError::PrestateDrift);
        }
        let impacts =
            normalized_id_set(host.compatible_present_devices(&self.driver_plan.source_inf)?)?;
        if impacts != self.driver_plan.impact_ids() {
            return Err(DriverStoreError::ImpactDrift);
        }
        let inventory = host.inventory()?;
        inventory.validate()?;
        for impact in &self.driver_plan.impacts {
            let current = current_binding(inventory.device(&impact.instance_id));
            if current != Some(impact.baseline.clone()) {
                return Err(DriverStoreError::PrestateDrift);
            }
            if host
                .resolve_published_package(&impact.baseline_package.published_inf)?
                .as_ref()
                != Some(&impact.baseline_package)
            {
                return Err(DriverStoreError::PrestateDrift);
            }
        }
        match &self.driver_plan.store_baseline {
            DriverStoreBaseline::Existing { package } => {
                if host
                    .resolve_published_package(&package.published_inf)?
                    .as_ref()
                    != Some(package)
                {
                    return Err(DriverStoreError::PrestateDrift);
                }
            }
            DriverStoreBaseline::Absent => {
                if host
                    .find_equivalent_package(
                        &self.driver_plan.source_inf,
                        std::slice::from_ref(&self.driver_plan.expected_signature.catalog_file),
                    )?
                    .is_some()
                {
                    return Err(DriverStoreError::PrestateDrift);
                }
            }
        }
        Ok(inventory)
    }

    fn validate_target_package<H: DriverHost>(
        &self,
        host: &H,
        package: &StoredDriverPackage,
    ) -> Result<(), DriverStoreError> {
        package.validate()?;
        if sha256_file(&package.driver_store_inf)? != self.driver_plan.source_inf_sha256 {
            return Err(DriverStoreError::StagedPackageMismatch);
        }
        let signature = host.verify_inf_signature(&package.driver_store_inf)?;
        if !signature_matches(&signature, &self.driver_plan.expected_signature) {
            return Err(DriverStoreError::StagedPackageMismatch);
        }
        if host
            .resolve_published_package(&package.published_inf)?
            .as_ref()
            != Some(package)
        {
            return Err(DriverStoreError::StagedPackageMismatch);
        }
        Ok(())
    }

    fn evaluate_forward(
        &self,
        before: &DriverInventory,
        after: &DriverInventory,
    ) -> Result<(bool, Option<String>), DriverStoreError> {
        let target = self.target_package.as_ref();
        let impact_ids = self.driver_plan.impact_ids();
        for device in &before.devices {
            let identity = device.instance_id.as_str().to_ascii_lowercase();
            if impact_ids.contains(&identity) {
                continue;
            }
            let Some(post) = after.device(device.instance_id.as_str()) else {
                return Ok((false, Some(device.instance_id.to_string())));
            };
            if current_binding(Some(device)) != current_binding(Some(post)) {
                return Ok((false, Some(device.instance_id.to_string())));
            }
        }
        if let Some(package) = target {
            for device in &after.devices {
                let identity = device.instance_id.as_str().to_ascii_lowercase();
                if !impact_ids.contains(&identity)
                    && active_published(device)
                        .is_some_and(|value| value.eq_ignore_ascii_case(&package.published_inf))
                {
                    return Ok((false, Some(device.instance_id.to_string())));
                }
            }
        }
        Ok((self.policy_satisfied_by_inventory(after), None))
    }

    fn policy_satisfied_by_inventory(&self, inventory: &DriverInventory) -> bool {
        let Some(target) = self.target_package.as_ref() else {
            return false;
        };
        for impact in &self.driver_plan.impacts {
            let Some(device) = inventory.device(&impact.instance_id) else {
                return false;
            };
            let current = current_binding(Some(device));
            let on_target = active_published(device)
                .is_some_and(|value| value.eq_ignore_ascii_case(&target.published_inf));
            let healthy = device.problem_code.is_none_or(|code| code == 0);
            let baseline_healthy = impact.baseline.problem_code.is_none_or(|code| code == 0);
            if on_target {
                if !healthy {
                    return false;
                }
            } else if current == Some(impact.baseline.clone()) {
                if !baseline_healthy {
                    return false;
                }
            } else {
                return false;
            }
        }
        match self.driver_plan.store_baseline {
            DriverStoreBaseline::Existing { .. } => true,
            DriverStoreBaseline::Absent => {
                let any_target = self.driver_plan.impacts.iter().any(|impact| {
                    inventory
                        .device(&impact.instance_id)
                        .and_then(active_published)
                        .is_some_and(|value| value.eq_ignore_ascii_case(&target.published_inf))
                });
                any_target || !package_in_use(inventory, &target.published_inf)
            }
        }
    }

    fn policy_observation<H: DriverHost>(&self, host: &H) -> Result<Observation, DriverStoreError> {
        let inventory = host.inventory()?;
        inventory.validate()?;
        let mut satisfied = self.policy_satisfied_by_inventory(&inventory);
        if matches!(self.driver_plan.store_baseline, DriverStoreBaseline::Absent) {
            if let Some(target) = self.target_package.as_ref() {
                let present = host
                    .resolve_published_package(&target.published_inf)?
                    .is_some();
                let any_target = package_in_use(&inventory, &target.published_inf);
                if present != any_target {
                    satisfied = false;
                }
            }
        }
        Ok(Observation {
            target: policy_target(&self.driver_plan.fingerprint()?),
            value: ObservedValue::Present(if satisfied {
                "satisfied".to_string()
            } else {
                "unsatisfied".to_string()
            }),
        })
    }

    fn impacted_binding_changed(&self, inventory: &DriverInventory) -> bool {
        self.driver_plan.impacts.iter().any(|impact| {
            current_binding(inventory.device(&impact.instance_id)) != Some(impact.baseline.clone())
        })
    }

    fn store_matches_baseline<H: DriverHost>(&self, host: &H) -> Result<bool, DriverStoreError> {
        match &self.driver_plan.store_baseline {
            DriverStoreBaseline::Existing { package } => Ok(host
                .resolve_published_package(&package.published_inf)?
                .as_ref()
                == Some(package)),
            DriverStoreBaseline::Absent => match self.target_package.as_ref() {
                Some(target) => Ok(host
                    .resolve_published_package(&target.published_inf)?
                    .is_none()),
                None => Ok(true),
            },
        }
    }

    fn restore_driver_store_if_possible<H: DriverHost>(
        &self,
        host: &H,
    ) -> Result<(), DriverStoreError> {
        match &self.driver_plan.store_baseline {
            DriverStoreBaseline::Existing { package } => {
                if host
                    .resolve_published_package(&package.published_inf)?
                    .as_ref()
                    == Some(package)
                {
                    Ok(())
                } else {
                    Err(DriverStoreError::DriverStoreRestoreFailure)
                }
            }
            DriverStoreBaseline::Absent => {
                let Some(target) = self.target_package.as_ref() else {
                    return Err(DriverStoreError::DriverStoreRestoreFailure);
                };
                let inventory = host.inventory()?;
                inventory.validate()?;
                if package_in_use(&inventory, &target.published_inf) {
                    return Err(DriverStoreError::DriverStoreRestoreFailure);
                }
                if host
                    .resolve_published_package(&target.published_inf)?
                    .is_some()
                {
                    host.remove_published_package(&target.published_inf)?;
                }
                if host
                    .resolve_published_package(&target.published_inf)?
                    .is_some()
                {
                    Err(DriverStoreError::DriverStoreRestoreFailure)
                } else {
                    Ok(())
                }
            }
        }
    }

    fn rollback_observations<H: DriverHost>(
        &self,
        host: &H,
    ) -> Result<Vec<Observation>, DriverStoreError> {
        let inventory = host.inventory()?;
        inventory.validate()?;
        let fingerprint = self.driver_plan.fingerprint()?;
        let mut observations = vec![Observation {
            target: store_target(&fingerprint),
            value: match &self.driver_plan.store_baseline {
                DriverStoreBaseline::Existing { package } => {
                    if host
                        .resolve_published_package(&package.published_inf)?
                        .as_ref()
                        == Some(package)
                    {
                        ObservedValue::Present(serde_json::to_string(package)?)
                    } else {
                        ObservedValue::Absent
                    }
                }
                DriverStoreBaseline::Absent => match self.target_package.as_ref() {
                    Some(target)
                        if host
                            .resolve_published_package(&target.published_inf)?
                            .is_some() =>
                    {
                        ObservedValue::Present(serde_json::to_string(target)?)
                    }
                    Some(_) => ObservedValue::Absent,
                    None => ObservedValue::Unavailable(
                        "target package identity was not recovered".to_string(),
                    ),
                },
            },
        }];
        for impact in &self.driver_plan.impacts {
            observations.push(Observation {
                target: binding_target(&impact.instance_id),
                value: match current_binding(inventory.device(&impact.instance_id)) {
                    Some(value) => ObservedValue::Present(serde_json::to_string(&value)?),
                    None => ObservedValue::Unavailable(
                        "captured device binding is not observable".to_string(),
                    ),
                },
            });
        }
        Ok(observations)
    }
}

fn current_binding(device: Option<&neo_device::DeviceRecord>) -> Option<DriverBindingBaseline> {
    let device = device?;
    Some(DriverBindingBaseline {
        binding: device.active_driver.clone()?,
        problem_code: device.problem_code,
    })
}

fn active_published(device: &neo_device::DeviceRecord) -> Option<&str> {
    device.active_driver.as_ref()?.published_name.as_deref()
}

fn package_in_use(inventory: &DriverInventory, published_inf: &str) -> bool {
    inventory.devices.iter().any(|device| {
        active_published(device).is_some_and(|value| value.eq_ignore_ascii_case(published_inf))
    })
}
