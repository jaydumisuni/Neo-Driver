#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"anchor mismatch in {path}: {old[:120]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


executor = Path("crates/neo-driverstore/src/executor.rs")
replace_once(
    executor,
    '''        let mut after = host.inventory()?;
        after.validate()?;
''',
    '''        let mut after = match host.inventory() {
            Ok(inventory) => inventory,
            Err(error) => {
                self.record_uncertain_apply_failure(
                    format!("post-mutation device inventory failed: {error}"),
                    reboot_required,
                )?;
                return self.validate();
            }
        };
        if let Err(error) = after.validate() {
            self.record_uncertain_apply_failure(
                format!("post-mutation device inventory was invalid: {error}"),
                reboot_required,
            )?;
            return self.validate();
        }
''',
)
replace_once(
    executor,
    '''                    } else {
                        after = host.inventory()?;
                        after.validate()?;
                    }
''',
    '''                    } else {
                        after = match host.inventory() {
                            Ok(inventory) => inventory,
                            Err(error) => {
                                self.record_uncertain_apply_failure(
                                    format!("post-cleanup device inventory failed: {error}"),
                                    reboot_required,
                                )?;
                                return self.validate();
                            }
                        };
                        if let Err(error) = after.validate() {
                            self.record_uncertain_apply_failure(
                                format!("post-cleanup device inventory was invalid: {error}"),
                                reboot_required,
                            )?;
                            return self.validate();
                        }
                    }
''',
)
replace_once(
    executor,
    '''        let store_changed = !self.store_matches_baseline(host)?;
        let machine_changed = binding_changed || store_changed;
''',
    '''        let store_changed = match self.store_matches_baseline(host) {
            Ok(matches_baseline) => !matches_baseline,
            Err(error) => {
                self.record_uncertain_apply_failure(
                    format!("post-mutation Driver Store probe failed: {error}"),
                    reboot_required,
                )?;
                return self.validate();
            }
        };
        let machine_changed = binding_changed || store_changed;
''',
)
replace_once(
    executor,
    '''        if outcome == ApplyOutcome::Success
            && self.transaction.stage() == TransactionStage::Verifying
        {
            let observation = self.policy_observation(host)?;
            self.transaction.verify_postconditions(vec![observation])?;
        }
        self.validate()
    }

    pub fn resume_after_reboot<H: DriverHost>(&mut self, host: &H) -> Result<(), DriverStoreError> {
''',
    '''        if outcome == ApplyOutcome::Success
            && self.transaction.stage() == TransactionStage::Verifying
        {
            self.verify_current(host)?;
        }
        self.validate()
    }

    pub fn verify_current<H: DriverHost>(&mut self, host: &H) -> Result<(), DriverStoreError> {
        self.validate()?;
        if self.transaction.stage() != TransactionStage::Verifying {
            return Err(DriverStoreError::SessionInvariantViolation);
        }
        let observation = self.policy_observation(host)?;
        self.transaction.verify_postconditions(vec![observation])?;
        self.validate()
    }

    pub fn resume_after_reboot<H: DriverHost>(&mut self, host: &H) -> Result<(), DriverStoreError> {
''',
)
replace_once(
    executor,
    '''        if self.transaction.stage() == TransactionStage::RollingBack {
            let observations = self.rollback_observations(host)?;
            self.transaction.verify_rollback(observations)?;
        }
        self.validate()
    }

    pub fn resume_after_rollback_reboot<H: DriverHost>(
''',
    '''        if self.transaction.stage() == TransactionStage::RollingBack {
            self.verify_rollback_current(host)?;
        }
        self.validate()
    }

    pub fn verify_rollback_current<H: DriverHost>(
        &mut self,
        host: &H,
    ) -> Result<(), DriverStoreError> {
        self.validate()?;
        if self.transaction.stage() != TransactionStage::RollingBack {
            return Err(DriverStoreError::SessionInvariantViolation);
        }
        let observations = self.rollback_observations(host)?;
        self.transaction.verify_rollback(observations)?;
        self.validate()
    }

    pub fn resume_after_rollback_reboot<H: DriverHost>(
''',
)
replace_once(
    executor,
    '''    fn preflight<H: DriverHost>(&self, host: &H) -> Result<DriverInventory, DriverStoreError> {
''',
    '''    fn record_uncertain_apply_failure(
        &mut self,
        detail: String,
        reboot_required: bool,
    ) -> Result<(), DriverStoreError> {
        self.transaction.record_apply_result(ApplyRecord {
            action_id: self.driver_plan.action_id.clone(),
            outcome: ApplyOutcome::Failure,
            detail,
            machine_changed: true,
            reboot_required,
        })?;
        Ok(())
    }

    fn preflight<H: DriverHost>(&self, host: &H) -> Result<DriverInventory, DriverStoreError> {
''',
)

tests = Path("crates/neo-driverstore/src/tests.rs")
replace_once(
    tests,
    '''    stage_calls: usize,
}
''',
    '''    stage_calls: usize,
    inventory_calls: usize,
    fail_inventory_call: Option<usize>,
}
''',
)
replace_once(
    tests,
    '''                stage_calls: 0,
            }),
''',
    '''                stage_calls: 0,
                inventory_calls: 0,
                fail_inventory_call: None,
            }),
''',
)
replace_once(
    tests,
    '''    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
        Ok(self.state.borrow().inventory.clone())
    }
''',
    '''    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
        let mut state = self.state.borrow_mut();
        state.inventory_calls += 1;
        if state.fail_inventory_call == Some(state.inventory_calls) {
            state.fail_inventory_call = None;
            return Err(DriverStoreError::Windows(
                "synthetic inventory failure".to_string(),
            ));
        }
        Ok(state.inventory.clone())
    }
''',
)
marker = '''#[test]
fn backend_failure_after_binding_change_routes_exact_rollback() {
'''
text = tests.read_text(encoding="utf-8")
if text.count(marker) != 1:
    raise SystemExit("executor recovery regression insertion anchor mismatch")
added = '''#[test]
fn post_mutation_inventory_failure_routes_conservative_rollback() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    fixture.host.configure(|state| {
        state.fail_inventory_call = Some(state.inventory_calls + 2);
    });
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RollingBack);
    session.rollback(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RolledBack);
}

#[test]
fn transient_verification_probe_can_be_retried() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    fixture.host.configure(|state| {
        state.fail_inventory_call = Some(state.inventory_calls + 3);
    });
    let error = session.apply(&fixture.host).unwrap_err();
    assert!(error.to_string().contains("synthetic inventory failure"));
    assert_eq!(session.transaction().stage(), TransactionStage::Verifying);
    session.verify_current(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::Complete);
}

'''
tests.write_text(text.replace(marker, added + marker, 1), encoding="utf-8")
