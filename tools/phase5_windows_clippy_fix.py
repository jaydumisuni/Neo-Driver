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
    '''        let mut signer = SP_INF_SIGNER_INFO_V2_W::default();
        signer.cbSize = std::mem::size_of::<SP_INF_SIGNER_INFO_V2_W>() as u32;
''',
    '''        let mut signer = SP_INF_SIGNER_INFO_V2_W {
            cbSize: std::mem::size_of::<SP_INF_SIGNER_INFO_V2_W>() as u32,
            ..Default::default()
        };
''',
)
replace_once(
    path,
    '''    let mut params = SP_DEVINSTALL_PARAMS_W::default();
    params.cbSize = std::mem::size_of::<SP_DEVINSTALL_PARAMS_W>() as u32;
''',
    '''    let mut params = SP_DEVINSTALL_PARAMS_W {
        cbSize: std::mem::size_of::<SP_DEVINSTALL_PARAMS_W>() as u32,
        ..Default::default()
    };
''',
)
replace_once(
    path,
    '''fn devinfo_data() -> SP_DEVINFO_DATA {
    let mut value = SP_DEVINFO_DATA::default();
    value.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
    value
}
''',
    '''fn devinfo_data() -> SP_DEVINFO_DATA {
    SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    }
}
''',
)
replace_once(
    path,
    '''fn drvinfo_data() -> SP_DRVINFO_DATA_V2_W {
    let mut value = SP_DRVINFO_DATA_V2_W::default();
    value.cbSize = std::mem::size_of::<SP_DRVINFO_DATA_V2_W>() as u32;
    value
}
''',
    '''fn drvinfo_data() -> SP_DRVINFO_DATA_V2_W {
    SP_DRVINFO_DATA_V2_W {
        cbSize: std::mem::size_of::<SP_DRVINFO_DATA_V2_W>() as u32,
        ..Default::default()
    }
}
''',
)
