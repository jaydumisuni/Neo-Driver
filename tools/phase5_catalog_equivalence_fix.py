#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"anchor mismatch in {path}: {old[:120]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


windows = Path("crates/neo-driverstore/src/windows.rs")
replace_once(
    windows,
    '''        let source_bytes = fs::read(source_inf)?;
        let source_signature = self.verify_inf_signature(source_inf)?;
''',
    '''        let source_bytes = fs::read(source_inf)?;
        let source_signature = self.verify_inf_signature(source_inf)?;
        let source_catalog = source_catalog_path(source_inf, &source_signature.catalog_file)?;
        let source_catalog_bytes = fs::read(&source_catalog)?;
''',
)
replace_once(
    windows,
    '''            if signature_matches(&candidate_signature, &source_signature) {
                if let Some(package) = self.resolve_published_package(name)? {
                    return Ok(Some(package));
                }
            }
''',
    '''            if signature_matches(&candidate_signature, &source_signature) {
                let candidate_catalog = path.with_extension("cat");
                let candidate_catalog_bytes = match fs::read(candidate_catalog) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };
                if candidate_catalog_bytes != source_catalog_bytes {
                    continue;
                }
                if let Some(package) = self.resolve_published_package(name)? {
                    return Ok(Some(package));
                }
            }
''',
)
replace_once(
    windows,
    '''fn windows_inf_dir() -> Result<PathBuf, DriverStoreError> {
''',
    '''fn source_catalog_path(inf: &Path, catalog_file: &str) -> Result<PathBuf, DriverStoreError> {
    let name = Path::new(catalog_file)
        .file_name()
        .ok_or(DriverStoreError::InvalidSignatureEvidence)?;
    let parent = inf.parent().ok_or(DriverStoreError::UnsafeInfPath)?;
    let catalog = parent.join(name);
    if !catalog.is_file() {
        return Err(DriverStoreError::InvalidSignatureEvidence);
    }
    Ok(catalog)
}

fn windows_inf_dir() -> Result<PathBuf, DriverStoreError> {
''',
)
replace_once(
    windows,
    '''    #[test]
    fn config_manager_status_failure_is_not_treated_as_healthy() {
''',
    '''    #[test]
    fn catalog_equivalence_requires_identical_bytes() {
        let root = std::env::temp_dir().join(format!(
            "neo-driverstore-catalog-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.cat");
        let candidate = root.join("candidate.cat");
        std::fs::write(&source, b"catalog-a").unwrap();
        std::fs::write(&candidate, b"catalog-b").unwrap();
        assert_ne!(std::fs::read(&source).unwrap(), std::fs::read(&candidate).unwrap());
        std::fs::write(&candidate, b"catalog-a").unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), std::fs::read(&candidate).unwrap());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn config_manager_status_failure_is_not_treated_as_healthy() {
''',
)

executor = Path("crates/neo-driverstore/src/executor.rs")
replace_once(
    executor,
    '''        match &self.driver_plan.store_baseline {
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
''',
    '''        let equivalent = host.find_equivalent_package(
            &self.driver_plan.source_inf,
            std::slice::from_ref(&self.driver_plan.expected_signature.catalog_file),
        )?;
        match &self.driver_plan.store_baseline {
            DriverStoreBaseline::Existing { package } => {
                if equivalent.as_ref() != Some(package) {
                    return Err(DriverStoreError::PrestateDrift);
                }
            }
            DriverStoreBaseline::Absent => {
                if equivalent.is_some() {
                    return Err(DriverStoreError::PrestateDrift);
                }
            }
        }
''',
)
replace_once(
    executor,
    '''        if host
            .resolve_published_package(&package.published_inf)?
            .as_ref()
            != Some(package)
        {
            return Err(DriverStoreError::StagedPackageMismatch);
        }
''',
    '''        if host
            .find_equivalent_package(
                &self.driver_plan.source_inf,
                std::slice::from_ref(&self.driver_plan.expected_signature.catalog_file),
            )?
            .as_ref()
            != Some(package)
        {
            return Err(DriverStoreError::StagedPackageMismatch);
        }
''',
)

review = Path("tools/phase5_static_review.py")
replace_once(
    review,
    '''                    "SetupGetInfDriverStoreLocationW",
                    "is_safe_published_name",
                    "StagedPackageMismatch",
''',
    '''                    "SetupGetInfDriverStoreLocationW",
                    "source_catalog_path",
                    "source_catalog_bytes",
                    "candidate_catalog_bytes",
                    "is_safe_published_name",
                    "StagedPackageMismatch",
''',
)
replace_once(
    review,
    '''            "staging captures and round-trips the exact Windows OEM published/package identity",
''',
    '''            "staging/equivalence uses binary-identical INF plus identical catalog bytes and round-trips the exact Windows OEM published/package identity",
''',
)
