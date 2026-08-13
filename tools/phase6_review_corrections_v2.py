from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {count}")
    return text.replace(old, new, 1)


probe_path = ROOT / "crates/neo-runtime-probe/src/lib.rs"
probe = probe_path.read_text(encoding="utf-8")
probe = replace_once(
    probe,
    ".filter(|item| !item.succeeded() && !known_absent(item))",
    ".filter(|item| !item.succeeded())",
    "warning fail-closed filter",
)
probe = replace_once(
    probe,
    "    if evidence.iter().all(|item| known_absent(item)) {\n        return observation(component, RuntimeState::Missing, source, vec![], None);\n    }\n",
    "",
    "registry false-missing branch",
)
probe = replace_once(
    probe,
    "        if let Some(release) = parse_reg_value(&evidence.stdout, \"Release\") {\n            if parse_reg_number(&release).is_some() {\n                return observation(\n                    RuntimeComponent::DotNetFramework4,\n                    RuntimeState::Installed,\n                    source,\n                    vec![format!(\"Release={release}\")],\n                    None,\n                );\n            }\n        }\n",
    "        if let Some(release_raw) = parse_reg_value(&evidence.stdout, \"Release\") {\n            if let Some(release) = parse_reg_number(&release_raw) {\n                if let Some(version) = netfx4_version(release) {\n                    return RuntimeObservation {\n                        component: RuntimeComponent::DotNetFramework4,\n                        state: RuntimeState::Installed,\n                        detected_version: Some(version.to_string()),\n                        source: source.to_string(),\n                        details: vec![format!(\"Release={release_raw}\")],\n                    };\n                }\n            }\n        }\n",
    "netfx version mapping",
)
probe = replace_once(
    probe,
    "    if known_absent(evidence) {\n        return observation(\n            RuntimeComponent::DotNetFramework4,\n            RuntimeState::Missing,\n            source,\n            vec![],\n            None,\n        );\n    }\n",
    "",
    "netfx false-missing branch",
)
probe = replace_once(
    probe,
    "    if evidence.iter().all(|item| known_absent(item)) {\n        return observation(\n            RuntimeComponent::WebView2,\n            RuntimeState::Missing,\n            source,\n            vec![],\n            None,\n        );\n    }\n",
    "",
    "webview false-missing branch",
)
probe = replace_once(
    probe,
    "        Some(FeatureState::Enabled) => observation(\n            component,\n            RuntimeState::Installed,\n            &source,\n            vec![\"State=Enabled\".to_string()],\n            None,\n        ),\n        Some(FeatureState::Disabled) => observation(\n            component,\n            RuntimeState::Missing,\n            &source,\n            vec![\"State=Disabled\".to_string()],\n            None,\n        ),\n",
    "        Some(FeatureState::Enabled) => observation(\n            component,\n            RuntimeState::Installed,\n            &source,\n            vec![\"State=Enabled\".to_string()],\n            None,\n        ),\n        Some(FeatureState::Disabled) => observation(\n            component,\n            RuntimeState::Missing,\n            &source,\n            vec![\"State=Disabled\".to_string()],\n            None,\n        ),\n        Some(FeatureState::EnablePending) => observation(\n            component,\n            RuntimeState::Partial,\n            &source,\n            vec![\"State=EnablePending\".to_string()],\n            Some(\"The feature is pending enablement and requires reboot completion before Neo can call it installed.\"),\n        ),\n        Some(FeatureState::DisablePending) => observation(\n            component,\n            RuntimeState::Partial,\n            &source,\n            vec![\"State=DisablePending\".to_string()],\n            Some(\"The feature is pending disablement and requires reboot completion before Neo can call it missing.\"),\n        ),\n",
    "pending feature classification",
)
probe = replace_once(
    probe,
    "enum FeatureState {\n    Enabled,\n    Disabled,\n    PayloadRemoved,\n}\n",
    "enum FeatureState {\n    Enabled,\n    Disabled,\n    EnablePending,\n    DisablePending,\n    PayloadRemoved,\n}\n",
    "pending feature enum",
)
probe = replace_once(
    probe,
    '            "enabled" | "enablepending" => Some(FeatureState::Enabled),\n            "disabled" | "disablepending" => Some(FeatureState::Disabled),\n',
    '            "enabled" => Some(FeatureState::Enabled),\n            "disabled" => Some(FeatureState::Disabled),\n            "enablepending" => Some(FeatureState::EnablePending),\n            "disablepending" => Some(FeatureState::DisablePending),\n',
    "pending feature parser",
)
probe = replace_once(
    probe,
    "    let runtime_paths: Vec<String> = python_list\n        .stdout\n        .lines()\n        .map(str::trim)\n        .filter(|line| line.to_ascii_lowercase().contains(\"python.exe\"))\n        .map(ToOwned::to_owned)\n        .collect();\n",
    "    let runtime_paths: Vec<String> = python_list\n        .stdout\n        .lines()\n        .map(str::trim)\n        .filter(|line| line.to_ascii_lowercase().contains(\"python.exe\"))\n        .map(ToOwned::to_owned)\n        .collect();\n    let runtime_versions = python_versions_from_listing(&python_list.stdout);\n",
    "python version extraction",
)
probe = replace_once(
    probe,
    "        return observation(RuntimeComponent::Python, state, source, details, None);\n",
    "        return RuntimeObservation {\n            component: RuntimeComponent::Python,\n            state,\n            detected_version: runtime_versions.first().cloned(),\n            source: source.to_string(),\n            details,\n        };\n",
    "python accurate detected version",
)
probe = replace_once(
    probe,
    "fn parse_reg_number(raw: &str) -> Option<u64> {\n",
    "fn python_versions_from_listing(output: &str) -> Vec<String> {\n    output\n        .lines()\n        .filter_map(|line| {\n            line.split_whitespace()\n                .find_map(|token| token.strip_prefix(\"-V:\"))\n                .map(|version| version.trim_end_matches('*').to_string())\n        })\n        .filter(|version| !version.is_empty())\n        .collect()\n}\n\nfn netfx4_version(release: u64) -> Option<&'static str> {\n    if release >= 533_320 {\n        Some(\"4.8.1 or later\")\n    } else if release >= 528_040 {\n        Some(\"4.8\")\n    } else if release >= 461_808 {\n        Some(\"4.7.2\")\n    } else if release >= 461_308 {\n        Some(\"4.7.1\")\n    } else if release >= 460_798 {\n        Some(\"4.7\")\n    } else if release >= 394_802 {\n        Some(\"4.6.2\")\n    } else if release >= 394_254 {\n        Some(\"4.6.1\")\n    } else if release >= 393_295 {\n        Some(\"4.6\")\n    } else if release >= 379_893 {\n        Some(\"4.5.2\")\n    } else if release >= 378_675 {\n        Some(\"4.5.1\")\n    } else if release >= 378_389 {\n        Some(\"4.5\")\n    } else {\n        None\n    }\n}\n\nfn parse_reg_number(raw: &str) -> Option<u64> {\n",
    "version helpers",
)
probe = replace_once(
    probe,
    "fn known_absent(evidence: &CommandEvidence) -> bool {\n    evidence.start_error.is_none() && evidence.exit_code == Some(1)\n}\n\n",
    "",
    "remove unsafe known_absent",
)
probe = replace_once(
    probe,
    "    fn vc_registry_known_absence_is_missing() {\n        let a = absent();\n        let b = absent();\n        let result = classify_registry_runtime(\n            RuntimeComponent::VcRedist2015PlusX86,\n            \"fixture\",\n            \"Version\",\n            &[&a, &b],\n        );\n        assert_eq!(result.state, RuntimeState::Missing);\n    }\n",
    "    fn vc_registry_failed_queries_are_unknown() {\n        let a = absent();\n        let b = absent();\n        let result = classify_registry_runtime(\n            RuntimeComponent::VcRedist2015PlusX86,\n            \"fixture\",\n            \"Version\",\n            &[&a, &b],\n        );\n        assert_eq!(result.state, RuntimeState::Unknown);\n    }\n",
    "registry regression",
)
probe = replace_once(
    probe,
    "        assert_eq!(parse_dism_feature_state(\"State : Surprise\\n\"), None);\n    }\n",
    "        assert_eq!(\n            parse_dism_feature_state(\"State : Enable Pending\\n\"),\n            Some(FeatureState::EnablePending)\n        );\n        assert_eq!(\n            parse_dism_feature_state(\"State : Disable Pending\\n\"),\n            Some(FeatureState::DisablePending)\n        );\n        assert_eq!(parse_dism_feature_state(\"State : Surprise\\n\"), None);\n    }\n\n    #[test]\n    fn pending_windows_features_are_partial_until_reboot() {\n        let enabling = success(\"Feature Name : NetFx3\\nState : Enable Pending\\n\");\n        let disabling = success(\"Feature Name : DirectPlay\\nState : Disable Pending\\n\");\n        assert_eq!(\n            classify_windows_feature(RuntimeComponent::DotNetFramework35, \"NetFx3\", &enabling).state,\n            RuntimeState::Partial\n        );\n        assert_eq!(\n            classify_windows_feature(RuntimeComponent::DirectPlay, \"DirectPlay\", &disabling).state,\n            RuntimeState::Partial\n        );\n    }\n\n    #[test]\n    fn netfx_release_maps_to_documented_version_threshold() {\n        let evidence = success(\"    Release    REG_DWORD    0x82405\\n\");\n        let result = classify_netfx4(&evidence);\n        assert_eq!(result.state, RuntimeState::Installed);\n        assert_eq!(result.detected_version.as_deref(), Some(\"4.8.1 or later\"));\n        assert!(result.details.iter().any(|detail| detail == \"Release=0x82405\"));\n    }\n",
    "feature and netfx regressions",
)
probe = replace_once(
    probe,
    "        assert_eq!(result.state, RuntimeState::Partial);\n        assert!(result\n",
    "        assert_eq!(result.state, RuntimeState::Partial);\n        assert_eq!(result.detected_version.as_deref(), Some(\"3.14\"));\n        assert!(result\n",
    "python version regression",
)
probe = replace_once(
    probe,
    "    fn webview_zero_version_is_not_installed() {\n        let zero = success(\"    pv    REG_SZ    0.0.0.0\\n\");\n        let missing = absent();\n        let result = classify_webview2(&[&zero, &missing]);\n        assert_eq!(result.state, RuntimeState::Broken);\n    }\n",
    "    fn webview_zero_version_is_not_installed() {\n        let zero = success(\"    pv    REG_SZ    0.0.0.0\\n\");\n        let missing = absent();\n        let result = classify_webview2(&[&zero, &missing]);\n        assert_eq!(result.state, RuntimeState::Broken);\n    }\n\n    #[test]\n    fn webview_failed_queries_are_unknown_not_missing() {\n        let first = absent();\n        let second = absent();\n        let result = classify_webview2(&[&first, &second]);\n        assert_eq!(result.state, RuntimeState::Unknown);\n    }\n",
    "webview fail-closed regression",
)
probe_path.write_text(probe, encoding="utf-8")

runtime_path = ROOT / "crates/neo-runtime/src/lib.rs"
runtime = runtime_path.read_text(encoding="utf-8")
runtime = replace_once(
    runtime,
    "        let chosen = match candidates.as_slice() {\n            [single] => Some(*single),\n",
    "        let chosen = match candidates.as_slice() {\n            [single] if single.dependencies.is_empty() && single.conflicts.is_empty() => Some(*single),\n            [single] => {\n                warnings.push(\n                    \"The compatible package has dependency/conflict edges; Phase 6 will not create standalone action authority until dependency-closure planning is proven.\".to_string(),\n                );\n                None\n            }\n",
    "dependency closure gate",
)
runtime = replace_once(
    runtime,
    "    fn non_runtime_binding_is_rejected() {\n",
    "    fn dependency_graph_never_becomes_standalone_action_authority() {\n        let dependency = package(\"runtime.dependency\");\n        let mut main = package(\"runtime.vc.x86\");\n        main.dependencies.push(\"runtime.dependency\".to_string());\n        let catalogue = Catalogue {\n            packages: vec![main, dependency],\n        };\n        let policy = RuntimePolicy {\n            bindings: vec![RuntimePackageBinding {\n                component: RuntimeComponent::VcRedist2015PlusX86,\n                package_id: \"runtime.vc.x86\".to_string(),\n            }],\n        };\n        let assessment = assess_runtime_profile(\n            RuntimeProfile::FreshWindows,\n            &inventory(RuntimeState::Missing),\n            &catalogue,\n            &policy,\n        )\n        .unwrap();\n        let item = assessment\n            .recommendations\n            .iter()\n            .find(|item| item.component == RuntimeComponent::VcRedist2015PlusX86)\n            .unwrap();\n        assert_eq!(item.verdict, EvidenceVerdict::Investigate);\n        assert!(item.action.is_none());\n        assert!(item\n            .warnings\n            .iter()\n            .any(|warning| warning.contains(\"dependency-closure\")));\n    }\n\n    #[test]\n    fn non_runtime_binding_is_rejected() {\n",
    "dependency regression",
)
runtime_path.write_text(runtime, encoding="utf-8")

review_path = ROOT / "tools/phase6_static_review.py"
review = review_path.read_text(encoding="utf-8")
review = replace_once(
    review,
    '            "package selection is architecture/build gated and ambiguous candidates fail closed",\n',
    '            "package selection is architecture/build gated, ambiguous candidates fail closed, and dependency/conflict graphs cannot become standalone action authority",\n',
    "lane9 detail",
)
review = replace_once(
    review,
    '                    "candidates.as_slice()",\n',
    '                    "candidates.as_slice()",\n                    "dependency-closure planning is proven",\n                    "dependency_graph_never_becomes_standalone_action_authority",\n',
    "lane9 dependency markers",
)
review = replace_once(
    review,
    "                    '\"Release\"',\n",
    "                    '\"Release\"',\n                    \"netfx4_version\",\n                    \"4.8.1 or later\",\n                    \"vc_registry_failed_queries_are_unknown\",\n                    \"webview_failed_queries_are_unknown_not_missing\",\n",
    "lane14 registry markers",
)
review = replace_once(
    review,
    "                    '\"/FeatureName:DirectPlay\"',\n",
    "                    '\"/FeatureName:DirectPlay\"',\n                    \"EnablePending\",\n                    \"DisablePending\",\n                    \"pending_windows_features_are_partial_until_reboot\",\n",
    "lane14 feature pending markers",
)
review = replace_once(
    review,
    '                    "pip_on_path",\n',
    '                    "pip_on_path",\n                    "python_versions_from_listing",\n',
    "lane15 python version marker",
)
review_path.write_text(review, encoding="utf-8")
