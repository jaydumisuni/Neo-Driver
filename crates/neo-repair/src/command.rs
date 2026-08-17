use crate::model::{FeatureDesiredState, SupportedWindowsFeature};
use crate::operation::RepairOperation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrustedProgram {
    Dism,
    Sfc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrustedCommand {
    pub(crate) program: TrustedProgram,
    pub(crate) args: Vec<String>,
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub(crate) fn component_store_inspection_command() -> TrustedCommand {
    TrustedCommand {
        program: TrustedProgram::Dism,
        args: args(&["/Online", "/Cleanup-Image", "/CheckHealth", "/English"]),
    }
}

pub(crate) fn system_files_inspection_command() -> TrustedCommand {
    TrustedCommand {
        program: TrustedProgram::Sfc,
        args: args(&["/verifyonly"]),
    }
}

pub(crate) fn feature_inspection_command(feature: SupportedWindowsFeature) -> TrustedCommand {
    TrustedCommand {
        program: TrustedProgram::Dism,
        args: vec![
            "/Online".to_string(),
            "/Get-FeatureInfo".to_string(),
            format!("/FeatureName:{}", feature.dism_name()),
            "/English".to_string(),
        ],
    }
}

pub(crate) fn operation_command(operation: RepairOperation) -> TrustedCommand {
    match operation {
        RepairOperation::RestoreComponentStore => TrustedCommand {
            program: TrustedProgram::Dism,
            args: args(&[
                "/Online",
                "/NoRestart",
                "/Cleanup-Image",
                "/RestoreHealth",
                "/English",
            ]),
        },
        RepairOperation::RepairSystemFiles => TrustedCommand {
            program: TrustedProgram::Sfc,
            args: args(&["/scannow"]),
        },
        RepairOperation::SetWindowsFeature { feature, desired } => TrustedCommand {
            program: TrustedProgram::Dism,
            args: vec![
                "/Online".to_string(),
                "/NoRestart".to_string(),
                match desired {
                    FeatureDesiredState::Enabled => "/Enable-Feature".to_string(),
                    FeatureDesiredState::Disabled => "/Disable-Feature".to_string(),
                },
                format!("/FeatureName:{}", feature.dism_name()),
                "/English".to_string(),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(program: TrustedProgram, values: &[&str]) -> TrustedCommand {
        TrustedCommand {
            program,
            args: args(values),
        }
    }

    #[test]
    fn trusted_command_contract_is_exact() {
        assert_eq!(
            component_store_inspection_command(),
            exact(
                TrustedProgram::Dism,
                &["/Online", "/Cleanup-Image", "/CheckHealth", "/English"],
            )
        );
        assert_eq!(
            system_files_inspection_command(),
            exact(TrustedProgram::Sfc, &["/verifyonly"])
        );
        assert_eq!(
            operation_command(RepairOperation::RestoreComponentStore),
            exact(
                TrustedProgram::Dism,
                &[
                    "/Online",
                    "/NoRestart",
                    "/Cleanup-Image",
                    "/RestoreHealth",
                    "/English",
                ],
            )
        );
        assert_eq!(
            operation_command(RepairOperation::RepairSystemFiles),
            exact(TrustedProgram::Sfc, &["/scannow"])
        );

        assert_eq!(
            SupportedWindowsFeature::all(),
            &[
                SupportedWindowsFeature::NetFx3,
                SupportedWindowsFeature::DirectPlay,
                SupportedWindowsFeature::HyperV,
                SupportedWindowsFeature::WindowsSubsystemLinux,
                SupportedWindowsFeature::VirtualMachinePlatform,
                SupportedWindowsFeature::WindowsSandbox,
            ]
        );

        for feature in SupportedWindowsFeature::all().iter().copied() {
            assert_eq!(
                feature_inspection_command(feature),
                TrustedCommand {
                    program: TrustedProgram::Dism,
                    args: vec![
                        "/Online".to_string(),
                        "/Get-FeatureInfo".to_string(),
                        format!("/FeatureName:{}", feature.dism_name()),
                        "/English".to_string(),
                    ],
                }
            );
            for desired in [FeatureDesiredState::Enabled, FeatureDesiredState::Disabled] {
                let command =
                    operation_command(RepairOperation::SetWindowsFeature { feature, desired });
                assert_eq!(command.program, TrustedProgram::Dism);
                assert_eq!(
                    command.args,
                    vec![
                        "/Online".to_string(),
                        "/NoRestart".to_string(),
                        match desired {
                            FeatureDesiredState::Enabled => "/Enable-Feature".to_string(),
                            FeatureDesiredState::Disabled => "/Disable-Feature".to_string(),
                        },
                        format!("/FeatureName:{}", feature.dism_name()),
                        "/English".to_string(),
                    ]
                );
                assert!(!command
                    .args
                    .iter()
                    .any(|arg| { matches!(arg.as_str(), "/Remove" | "/Source" | "/LimitAccess") }));
            }
        }
    }
}
