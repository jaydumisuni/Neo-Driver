#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"anchor mismatch in {path}: {old[:120]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


path = Path("crates/neo-driverstore/src/windows.rs")
replace_once(
    path,
    '''    SetupUninstallOEMInfW, SetupVerifyInfFileW, CM_DEVNODE_STATUS_FLAGS, CM_PROB, CR_SUCCESS,
''',
    '''    SetupUninstallOEMInfW, SetupVerifyInfFileW, CM_DEVNODE_STATUS_FLAGS, CM_PROB, CONFIGRET,
    CR_SUCCESS,
''',
)
replace_once(
    path,
    '''            let problem_code = problem_code(&data);
''',
    '''            let problem_code = problem_code(&data)?;
''',
)
replace_once(
    path,
    '''fn problem_code(data: &SP_DEVINFO_DATA) -> Option<u32> {
    let mut status = CM_DEVNODE_STATUS_FLAGS(0);
    let mut problem = CM_PROB(0);
    let result = unsafe { CM_Get_DevNode_Status(&mut status, &mut problem, data.DevInst, 0) };
    if result == CR_SUCCESS && problem.0 != 0 {
        Some(problem.0)
    } else {
        None
    }
}
''',
    '''fn problem_code(data: &SP_DEVINFO_DATA) -> Result<Option<u32>, DriverStoreError> {
    let mut status = CM_DEVNODE_STATUS_FLAGS(0);
    let mut problem = CM_PROB(0);
    let result = unsafe { CM_Get_DevNode_Status(&mut status, &mut problem, data.DevInst, 0) };
    decode_problem_code(result, problem)
}

fn decode_problem_code(
    result: CONFIGRET,
    problem: CM_PROB,
) -> Result<Option<u32>, DriverStoreError> {
    if result != CR_SUCCESS {
        return Err(DriverStoreError::Windows(format!(
            "CM_Get_DevNode_Status failed: CONFIGRET {}",
            result.0
        )));
    }
    Ok((problem.0 != 0).then_some(problem.0))
}
''',
)
replace_once(
    path,
    '''fn is_safe_published_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    !value.contains(['\\\\', '/'])
        && lower.starts_with("oem")
        && lower.ends_with(".inf")
        && lower[3..lower.len() - 4]
            .chars()
            .all(|character| character.is_ascii_digit())
}
''',
    '''fn is_safe_published_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if value.contains(['\\\\', '/']) || !lower.starts_with("oem") || !lower.ends_with(".inf") {
        return false;
    }
    let digits = &lower[3..lower.len() - 4];
    !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
}

#[cfg(test)]
mod windows_tests {
    use super::*;

    #[test]
    fn published_name_requires_numeric_oem_index() {
        assert!(is_safe_published_name("oem0.inf"));
        assert!(is_safe_published_name("OEM42.INF"));
        assert!(!is_safe_published_name("oem.inf"));
        assert!(!is_safe_published_name("oemx.inf"));
        assert!(!is_safe_published_name(r"sub\\oem1.inf"));
    }

    #[test]
    fn config_manager_status_failure_is_not_treated_as_healthy() {
        assert_eq!(decode_problem_code(CR_SUCCESS, CM_PROB(0)).unwrap(), None);
        assert_eq!(decode_problem_code(CR_SUCCESS, CM_PROB(10)).unwrap(), Some(10));
        let error = decode_problem_code(CONFIGRET(13), CM_PROB(0)).unwrap_err();
        assert!(error.to_string().contains("CONFIGRET 13"));
    }
}
''',
)
