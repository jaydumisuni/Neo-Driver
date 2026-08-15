use crate::{DebloatPlanError, ExactAppxInventory, ExactPackageDependency, ExactPackageIdentity};
use windows::core::HSTRING;
use windows::ApplicationModel::Package;
use windows::Management::Deployment::PackageManager;

pub(crate) fn scan_native_inventory() -> Result<ExactAppxInventory, DebloatPlanError> {
    let manager = PackageManager::new().map_err(native_error("create PackageManager"))?;

    let mut current_user = manager
        .FindPackagesByUserSecurityId(&HSTRING::new())
        .map_err(native_error("enumerate current-user packages"))?
        .into_iter()
        .map(|package| package_identity(&package))
        .collect::<Result<Vec<_>, _>>()?;

    let mut provisioned = manager
        .FindProvisionedPackages()
        .map_err(native_error("enumerate provisioned packages"))?
        .into_iter()
        .map(|package| package_identity(&package))
        .collect::<Result<Vec<_>, _>>()?;

    current_user.sort_by(|left, right| {
        left.full_name
            .to_ascii_lowercase()
            .cmp(&right.full_name.to_ascii_lowercase())
    });
    provisioned.sort_by(|left, right| {
        left.full_name
            .to_ascii_lowercase()
            .cmp(&right.full_name.to_ascii_lowercase())
    });

    ExactAppxInventory::new(
        current_user,
        provisioned,
        "neo-debloat-plan:Windows.Management.Deployment.PackageManager",
    )
}

fn package_identity(package: &Package) -> Result<ExactPackageIdentity, DebloatPlanError> {
    let id = package.Id().map_err(native_error("read Package.Id"))?;
    let mut dependencies = package
        .Dependencies()
        .map_err(native_error("read Package.Dependencies"))?
        .into_iter()
        .map(|dependency| {
            let dependency_id = dependency
                .Id()
                .map_err(native_error("read dependency Package.Id"))?;
            Ok(ExactPackageDependency {
                name: dependency_id
                    .Name()
                    .map_err(native_error("read dependency name"))?
                    .to_string_lossy(),
                full_name: dependency_id
                    .FullName()
                    .map_err(native_error("read dependency full name"))?
                    .to_string_lossy(),
                family_name: dependency_id
                    .FamilyName()
                    .map_err(native_error("read dependency family name"))?
                    .to_string_lossy(),
            })
        })
        .collect::<Result<Vec<_>, DebloatPlanError>>()?;
    dependencies.sort_by(|left, right| {
        left.full_name
            .to_ascii_lowercase()
            .cmp(&right.full_name.to_ascii_lowercase())
    });

    Ok(ExactPackageIdentity {
        name: id
            .Name()
            .map_err(native_error("read package name"))?
            .to_string_lossy(),
        full_name: id
            .FullName()
            .map_err(native_error("read package full name"))?
            .to_string_lossy(),
        family_name: id
            .FamilyName()
            .map_err(native_error("read package family name"))?
            .to_string_lossy(),
        is_framework: package
            .IsFramework()
            .map_err(native_error("read package framework flag"))?,
        is_resource: package
            .IsResourcePackage()
            .map_err(native_error("read package resource flag"))?,
        is_bundle: package
            .IsBundle()
            .map_err(native_error("read package bundle flag"))?,
        is_optional: package
            .IsOptional()
            .map_err(native_error("read package optional flag"))?,
        dependencies,
    })
}

fn native_error(operation: &'static str) -> impl FnOnce(windows::core::Error) -> DebloatPlanError {
    move |error| DebloatPlanError::NativeInventory(format!("{operation}: {error}"))
}
