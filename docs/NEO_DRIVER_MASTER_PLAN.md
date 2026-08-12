# Neo Driver — Master Plan / Architecture Freeze

**Status:** PLANNING FREEZE  
**Implementation status:** NOT STARTED  
**Purpose of this document:** Canonical source of truth for Neo Driver implementation.  

## Anti-drift rule

Implementation must follow this document. A material change to scope, architecture, safety policy, UX authority, package policy, execution doctrine, or proof requirements must be explicitly approved and written back into this document before implementation continues.

No silent substitutions. No "close enough" implementation. No skipped requirements.

---

# 1. Product definition

**Neo Driver** is a model-free Windows setup, driver, runtime, gaming prerequisite, technician workstation, debloat, tweak, repair, and recovery suite.

The product goal is the convenience of DriverPack / Driver Booster / WinUtil-class tools, but with stronger machine understanding, evidence, manual authority, rollback, verification, and technician-level control.

Neo Driver should be usable for:

- fresh Windows setup;
- missing-driver installation;
- wrong/broken/generic-driver repair;
- gaming-PC preparation;
- legacy-game prerequisite repair;
- technician / phone-repair workstation setup;
- runtime/framework setup;
- Windows debloat;
- Windows behaviour/UI/performance/privacy tweaks;
- Windows feature management;
- common repair workflows;
- manual expert driver/package work;
- offline workshop deployment.

The final product should be accessible both as an EXE GUI and a terminal command, backed by the same core engine.

---

# 2. Non-negotiable product laws

## 2.1 Model-free core

Neo must work correctly without an LLM, cloud model, AI account, Cloudflare AI, Hunter, OpenAI, or local model.

Neo intelligence comes from deterministic evidence:

- hardware IDs;
- compatible IDs;
- SUBSYS/OEM identity;
- Windows version/build/architecture;
- INF metadata;
- driver rank;
- signatures/catalogues;
- device state/problem codes;
- Driver Store state;
- runtime/package detection;
- dependency/conflict rules;
- security state;
- known-good and known-bad verified outcomes;
- package manifests;
- exact pre/post machine state.

Optional AI may be added later for explanation/research only. It must never become required for safe matching, planning, installation, rollback, or verification.

## 2.2 Manual authority

Neo may detect, rank, recommend, preselect, explain, and build a plan. Neo must not force a driver, runtime, application, tweak, debloat action, or security change merely because it recommends it.

Every installable/changeable item must be reviewable and individually selectable where practical.

"Recommended" or "baseline essential" means selected by default in the appropriate profile, not forced.

## 2.3 Explain before change

Neo must show what it intends to change, why, expected side effects, reboot requirements, and risk before applying material changes.

## 2.4 Capture actual state

Before a reversible change, Neo records the actual current state of the machine. Rollback restores the captured state rather than assuming a generic original value.

## 2.5 Verify after change

Installer exit code 0 is not proof. Neo re-probes and verifies the intended postcondition.

## 2.6 Fail closed on uncertainty

Unknown package compatibility, unsupported architecture/build, invalid signature/hash, unresolved critical conflict, or unresolved high-risk state must not become an automatic PASS.

## 2.7 Expert mode means more authority, not less evidence

Experts may access deeper controls. Neo still records evidence, warns about risk, and verifies outcomes.

---

# 3. Three-user interaction model

Neo is one product with one engine and three interaction depths.

## 3.1 Beginner — "Guide me"

Target user: little/no PC knowledge.

Neo speaks in outcomes rather than low-level identifiers.

Example:

- "3 important drivers are missing";
- "Your Wi-Fi, graphics and motherboard support need drivers";
- "Neo recommends 7 components and expects one reboot".

Beginner actions:

- Set up this PC;
- Fix a problem;
- Prepare for gaming;
- Safe cleanup;
- Recommended setup;
- Review before installing.

Beginner mode still permits deselection and "Show advanced details".

## 3.2 Standard — "Show recommendations"

Target user: understands common PC concepts such as GPU, chipset, DirectX, VC++, drivers.

Expose:

- installed/recommended version;
- reason for recommendation;
- source/vendor;
- health state;
- update/repair/reinstall/rollback/ignore/pin actions.

## 3.3 Expert / Technician — "Full control"

Expose low-level evidence and actions:

- Hardware ID / compatible IDs;
- SUBSYS / REV;
- INF;
- provider/date/version;
- rank;
- signer/catalogue;
- Driver Store package;
- service/filter stack;
- problem code;
- binding/history;
- source/OEM match;
- OS/build/architecture compatibility;
- reboot/security requirements.

Expert actions may include:

- stage/install INF;
- install local folder/ZIP/package;
- export/backup driver;
- rollback;
- remove stale package;
- re-enumerate/restart device;
- inspect filters/services;
- compare candidates;
- search by hardware ID;
- manual package selection.

---

# 4. First-launch intent model

Neo should first ask what the user is trying to accomplish, not dump the full control surface.

Primary intents:

- Set up this PC
- Fix a problem
- Install drivers
- Prepare for gaming
- Prepare technician tools
- Improve Windows
- Clean/debloat Windows
- Repair devices/drivers
- Advanced / I know what I need

Then interaction depth:

- Guide me
- Show recommendations
- Full control

The user can change depth at any time.

---

# 5. Main product sections

1. **Dashboard** — full machine scan, health, recommendations.
2. **Drivers** — PC/OEM/device drivers.
3. **Runtimes** — VC++, .NET, Python, DirectX and related prerequisites.
4. **Gaming** — GPU stack, game runtimes, legacy compatibility.
5. **Device Lab** — phone/service drivers and USB transport stacks.
6. **Debloat** — selectable Windows/AppX removal.
7. **Tweaks** — Windows UI/behaviour/performance/privacy/configuration.
8. **Repair** — diagnostics and repair workflows.
9. **Windows Features** — .NET 3.5, DirectPlay, Hyper-V, WSL, Sandbox, etc.
10. **Tools** — curated optional technician/gaming/diagnostic software.
11. **Manual** — direct package/driver/tweak operations.
12. **History & Restore** — every Neo mission/change and rollback path.
13. **Settings** — cache, offline sources, CLI integration, policy.

---

# 6. System X-Ray

Neo starts from evidence and builds a machine profile.

## 6.1 Hardware inventory

- CPU;
- GPU(s);
- motherboard;
- chipset/platform;
- BIOS/UEFI;
- RAM;
- storage/NVMe/SATA/RAID controllers;
- Ethernet;
- Wi-Fi;
- Bluetooth;
- audio;
- USB controllers/hubs;
- USB-C / Thunderbolt / USB4 where applicable;
- PCI devices;
- monitors;
- touch;
- cameras;
- biometrics;
- printers/scanners;
- unknown devices;
- disabled devices;
- problem devices.

## 6.2 Windows state

- edition/version/build;
- architecture;
- UEFI/Legacy;
- Secure Boot;
- TPM;
- Core Isolation / Memory Integrity / VBS;
- Hyper-V state;
- Test Signing;
- driver-signature/code-integrity relevant state;
- Developer Mode;
- Windows Update state;
- pending reboot;
- restore/recovery availability.

## 6.3 Driver state

- present devices;
- active driver/INF;
- provider/version/date;
- signer/catalogue state;
- Driver Store packages;
- generic Microsoft fallbacks;
- problem codes;
- service/filter relationships;
- stale/ghost/duplicate devices where relevant.

## 6.4 Runtime state

- DirectX capability;
- DirectX legacy components;
- Visual C++ runtime state;
- .NET Framework;
- modern .NET / Desktop Runtime;
- XNA;
- OpenAL;
- PhysX where applicable;
- Python / launcher / pip / PATH health;
- WebView2;
- Java later if justified;
- Vulkan/OpenCL capability/runtime where relevant.

## 6.5 Technician/device stack

- ADB;
- Fastboot;
- Apple drivers;
- Samsung;
- MTK;
- Qualcomm;
- SPD/Unisoc;
- Huawei;
- Exynos/EUB;
- serial/COM;
- WinUSB;
- UsbDk;
- libusb variants/filter state.

---

# 7. Driver decision model

Neo must not treat "newer version" as automatically better.

Hard rejection gates first:

- wrong architecture;
- unsupported Windows build;
- invalid/mismatched package hash;
- incompatible hardware;
- invalid/blocked package state;
- explicit known-bad rule.

Surviving candidates are ranked using evidence such as:

1. exact hardware ID;
2. SUBSYS/OEM identity;
3. compatible IDs;
4. device class;
5. Windows build;
6. architecture;
7. OEM/platform specificity;
8. current driver rank;
9. signature/catalogue;
10. version/date where relevant;
11. known-good verified history;
12. known conflicts/regressions.

User-facing recommendation states:

- REQUIRED;
- REPAIR;
- RECOMMENDED;
- HEALTHY;
- OPTIONAL UPDATE;
- OPTIONAL COMPONENT;
- OEM PREFERRED;
- GENERIC AVAILABLE;
- CONFLICT;
- UNSUPPORTED;
- DO NOT TOUCH;
- UNKNOWN.

Internal evidence verdicts may use:

- CERTIFIED;
- PROVISIONAL;
- INVESTIGATE;
- REJECTED.

Neo must be willing to recommend leaving a healthy driver alone.

---

# 8. Driver coverage

## Platform / chipset

- Intel chipset/platform;
- AMD chipset/platform;
- Intel MEI;
- AMD PSP;
- Serial IO;
- platform/ACPI/system-interface devices;
- OEM-specific system drivers.

## Graphics

- NVIDIA;
- AMD Radeon;
- Intel Arc/UHD/Iris;
- hybrid graphics.

## Audio

- Realtek;
- Intel SST;
- AMD/NVIDIA HDMI audio;
- USB audio;
- OEM audio packages.

## Networking

- Ethernet;
- Wi-Fi;
- Bluetooth;
- WWAN where applicable.

## Storage

- NVMe;
- AHCI/SATA;
- RAID;
- Intel RST/VMD;
- AMD RAID;
- relevant storage filters.

## Peripheral/platform

- HID;
- touchpads;
- touchscreen;
- camera;
- card reader;
- fingerprint/biometric;
- sensors;
- controller/gamepad;
- USB/USB-C/Thunderbolt/USB4;
- printers/scanners where appropriate.

**Firmware/BIOS:** detection may exist, but automatic BIOS/firmware flashing is not part of normal driver-update flow. It is high-risk/manual-only if added later.

---

# 9. Runtimes

Runtimes share the Neo UI but do not use the same install logic as INF drivers.

## 9.1 Visual C++ baseline

Neo v1 normal floor:

- Visual C++ **2015+ x86**;
- Visual C++ **2015+ x64**.

Use the current supported v14 redistributable family rather than pretending every modern Visual Studio year needs a separate package.

Older pre-2015 redistributables may be added under **Legacy Runtimes** when a verified application/game dependency requires them.

## 9.2 DirectX

Neo should distinguish current Windows DirectX capability from legacy side-by-side components.

For Fresh Windows and Gaming profiles, **Microsoft DirectX End-User Runtimes (June 2010)** is a baseline recommended component because it supplies legacy D3DX9/10/11, XAudio 2.7, XInput 1.3, and XACT-era dependencies used by older games/software without replacing modern Windows DirectX.

It may be preselected but remains deselectable.

## 9.3 .NET

Detect/manage where appropriate:

- .NET Framework 3.5 Windows feature;
- .NET Framework 4.x state/repair;
- modern .NET Runtime;
- .NET Desktop Runtime.

## 9.4 Python

Neo itself must not require Python.

Neo may detect/install/repair Python for the user:

- installed versions;
- x86/x64;
- Python Launcher;
- Store aliases;
- PATH;
- pip;
- venv support;
- user/system install distinction.

Do not install another Python merely because PATH is broken; offer repair when appropriate.

## 9.5 Gaming/legacy runtimes

Plan for:

- XNA Framework 4.0 Refresh;
- OpenAL;
- PhysX;
- PhysX Legacy where justified;
- DirectPlay;
- legacy VC++ where verified necessary.

---

# 10. Gaming section

Neo Gaming should provide a readiness view rather than only package installation.

Inspect:

- GPU and driver health;
- DirectX capability;
- DirectX legacy components;
- Visual C++ x86/x64;
- Vulkan/OpenCL support/runtime where meaningful;
- XNA;
- OpenAL;
- PhysX;
- .NET requirements;
- controller/HID support;
- gaming services where relevant.

Gaming profile must not debloat gaming dependencies.

A game/runtime repair must resolve the owning package/dependency, not download random DLL files.

---

# 11. Device Lab

Device Lab provides technician/service-driver setup.

Planned coverage:

- ADB;
- Fastboot;
- Google/Android USB;
- Apple Mobile Device / Recovery / DFU-related Windows drivers;
- Samsung USB;
- Qualcomm QDLoader/EDL;
- MediaTek VCOM and relevant stacks;
- SPD/Unisoc;
- Huawei COM/service drivers;
- Exynos EUB;
- HTC/LG/BlackBerry where supported;
- UsbDk;
- WinUSB/libusb stacks where justified.

Neo must include a **USB Stack Conflict Guard**. Before replacing a USB binding/filter, know:

- device instance;
- VID/PID/interface;
- current driver;
- class;
- service;
- upper/lower filters;
- target driver;
- expected effect.

No blanket filter/WinUSB replacement policy.

---

# 12. Debloat engine

Debloat is first-class, not an arbitrary PowerShell list.

Each removable component should include:

- package identity;
- installed/provisioned state;
- description;
- category;
- dependency/side-effect information;
- Store/restore source where applicable;
- current version;
- restore method.

Debloat classes:

- SAFE OPTIONAL;
- FEATURE DEPENDENT;
- DEPENDENCY SENSITIVE;
- PROTECTED / MANUAL ONLY.

Beginner "Safe Cleanup" selects only low-impact items.

Gaming and Technician profiles must preserve components needed by those workflows.

Before removal, capture actual package/provisioning state. History should offer a restore path when technically possible.

---

# 13. Tweaks engine

Borrow breadth and catalogue concepts from WinUtil, but re-engineer actions into typed, transactional Neo operations rather than unrestricted arbitrary scripts.

Examples:

- classic Windows 11 right-click menu;
- Explorer behaviour;
- file extensions/hidden files;
- taskbar;
- Start menu;
- Widgets;
- Search/Bing integration;
- Game Mode;
- mouse acceleration;
- telemetry/privacy preferences;
- OneDrive options;
- power behaviour;
- services;
- networking;
- selected Windows AI/Copilot-related OS surfaces where applicable.

Every tweak should expose:

- current state;
- desired state;
- what changes;
- benefit/trade-off;
- risk;
- admin requirement;
- Explorer restart/sign-out/reboot requirement;
- reversible status;
- verification rule;
- rollback rule.

Preferred typed operations:

- RegistrySet;
- RegistryDelete;
- ServiceStartup;
- AppxRemove/AppxRestore;
- FeatureEnable/Disable;
- ScheduledTaskEnable/Disable;
- ExplorerRestart;
- BCDChange;
- PackageInstall.

Exceptional command/script adapters must be tightly bounded and audited.

Risk levels:

- LOW;
- NORMAL;
- ELEVATED;
- HIGH;
- EXPERT.

---

# 14. Repair section

Repair workflows should follow Detect → Explain → Plan → Apply → Verify.

Planned areas:

- Driver Store/PnP repair;
- device re-enumeration;
- Windows component store / DISM;
- SFC;
- Windows Update reset/repair;
- networking reset/repair;
- Winget repair;
- AppX repair;
- Windows Features;
- restore/recovery state.

Symptom-driven entry points may include:

- Wi-Fi doesn't work;
- no sound;
- GPU problems;
- Bluetooth missing;
- unknown device;
- USB device not detected;
- phone driver problem;
- game won't launch;
- DirectX/runtime error;
- driver install failed;
- Windows keeps installing the wrong driver.

Neo translates symptom into deterministic investigation paths.

---

# 15. Windows Features

Manage supported optional features with explicit current/desired state and reboot implications.

Candidates include:

- .NET Framework 3.5;
- DirectPlay / legacy components;
- Hyper-V;
- WSL;
- Windows Sandbox;
- other justified Windows features.

---

# 16. Optional Tools catalogue

Neo may provide a curated manual Tools section for useful setup/diagnostic applications, using package-manager/vendor identifiers rather than becoming a general software store.

Possible categories:

- Technician;
- Gaming;
- Diagnostics;
- Development;
- General Setup.

Potential examples include DDU, CPU-Z, CrystalDiskInfo, Autoruns, 7-Zip, DISMTools, browsers, and developer utilities.

Everything remains user-selected.

---

# 17. Profiles

Profiles determine recommendations/preselection, not forced execution.

## Fresh Windows

Typical recommendations:

- chipset/platform;
- network;
- graphics;
- audio;
- USB/Bluetooth;
- VC++ 2015+ x86/x64;
- DirectX legacy;
- required .NET/WebView components.

## Gaming PC

Typical recommendations:

- appropriate GPU/chipset/audio/network drivers;
- VC++ 2015+ x86/x64;
- DirectX legacy;
- detected gaming dependencies;
- gaming-safe tweaks.

Optional:

- XNA;
- OpenAL;
- PhysX Legacy;
- .NET 3.5;
- DirectPlay.

## Technician / Phone Repair

Typical recommendations:

- platform drivers;
- VC++;
- .NET;
- ADB/Fastboot.

User-selectable technician stacks:

- Apple;
- Samsung;
- MTK;
- Qualcomm;
- SPD/Unisoc;
- Huawei;
- Exynos;
- UsbDk/WinUSB/libusb where justified;
- Python.

## Developer

Drivers plus development runtimes/tools as selected.

## Debloat Light / Custom

Low-impact or fully user-selected cleanup.

## Repair

Diagnostics first, then targeted repairs.

## Custom

No hidden bulk actions.

Profiles should eventually be saveable/exportable for workshop reuse.

---

# 18. Security-state engine

Neo must distinguish separate Windows security states rather than present a fake single "driver signature on/off" switch.

Track relevant state such as:

- administrator/elevation;
- Secure Boot;
- Test Signing;
- Memory Integrity/HVCI;
- code-integrity relevant state;
- pending reboot;
- BitLocker/recovery implications where relevant.

Scanning should not modify security configuration.

If a selected package genuinely requires a change:

1. identify the exact blocker;
2. explain the exact change and risk;
3. request explicit user approval;
4. elevate through visible UAC when required;
5. capture actual current state;
6. create a persistent mission checkpoint;
7. apply the minimum required change;
8. read back/verify it;
9. reboot;
10. automatically resume the mission;
11. re-probe before continuing;
12. install/verify the package;
13. restore the previous security state when appropriate and approved;
14. verify restoration.

Neo should not casually use weaker BCD/integrity settings when Test Signing or another narrower path is sufficient.

If a state was already enabled before Neo, Neo must not disable it simply because the mission finished.

---

# 19. Reboot/resume missions

Any workflow crossing a reboot must be persistent and restart-safe.

Mission stages should record:

- requested action;
- baseline state;
- completed stages;
- expected state after reboot;
- pending stage;
- verification predicates;
- restoration obligations.

After reboot, Neo reopens/resumes the mission, re-probes the system, and only continues if the expected state is actually present.

Never assume a reboot succeeded.

---

# 20. Transaction / rollback engine

Every material Neo change is a transaction where technically possible.

Generic lifecycle:

1. CAPTURE CURRENT STATE
2. BUILD EXACT PLAN
3. USER AUTHORITY
4. BACKUP/EXPORT WHERE APPLICABLE
5. APPLY
6. RE-PROBE
7. VERIFY
8. COMPLETE OR ROLLBACK/RECOVER
9. RECORD HISTORY/EVIDENCE

Driver baseline may include:

- device instance;
- current INF;
- version/provider/signer;
- service/filter stack;
- Driver Store package;
- hardware IDs;
- exported driver package where practical.

Tweak rollback restores captured values/state, not a hard-coded presumed default.

AppX rollback records the exact previous package/provisioning state and restore source where possible.

---

# 21. Verification rules

## Driver

Verify at minimum where applicable:

- device still present;
- expected INF active;
- expected version/provider;
- device problem code healthy;
- expected services active;
- no unexpected duplicate/conflict introduced;
- reboot obligation satisfied.

## Runtime

Verify installed package/version/architecture and appropriate detection predicates.

## Python

Verify selected version, launcher, pip, PATH/aliases where applicable.

## Tweak

Verify actual registry/service/feature/task state.

## Debloat

Verify package/provisioning state and record restore capability.

No mission is COMPLETE solely because an installer returned success.

---

# 22. Package catalogue and manifests

Every usable package/action becomes a structured Neo manifest.

Driver/package fields should include where applicable:

- neo package ID;
- type/category/vendor/name;
- version/date;
- architecture;
- supported Windows builds;
- hardware/compatible IDs;
- class/Class GUID;
- INF/catalogue files;
- source/source release;
- SHA-256;
- signature/signer;
- install method;
- silent/manual arguments;
- dependencies;
- conflicts;
- admin/reboot requirements;
- Test Signing/security requirements;
- detection rules;
- verification rules;
- rollback rules;
- risk;
- maturity;
- known-good evidence;
- known-bad evidence;
- redistribution/licensing policy.

Redistribution policy should distinguish at least:

- allowed;
- vendor-download-only;
- unknown.

Neo must not mirror/repackage software where redistribution rights are not established.

---

# 23. Source priority

Candidate source preference should generally be:

1. exact OEM source;
2. Microsoft/Windows source where appropriate;
3. chip/device manufacturer;
4. verified Neo/TTG catalogue mirror;
5. known-compatible generic package.

A newer generic package does not automatically beat a healthy exact OEM package.

---

# 24. Existing donor/source repositories

## Driver source vaults

- `jaydumisuni/android-drivers`
- `jaydumisuni/Exynos-driver`
- `jaydumisuni/Apple-windows-drivers`
- `jaydumisuni/TechGuyDrivers`

These are package/source vaults, not Neo architecture. Release assets must be inventoried and classified before exposure in Neo.

Pipeline:

GitHub/source asset → metadata → hash → extract → classify → INF/package inspect → signature verify → manifest → conflict analysis → Neo catalogue.

Mixed archives must not cause APK/XML/tool assets to appear as Windows drivers.

## Sergeant (`jaydumisuni/Sergeant`)

Borrow:

- mission/capability/evidence/gate thinking;
- fail-closed boundaries;
- officers/private-force discipline;
- recursive 20/2 execution method;
- final proof discipline;
- verified experience principles.

Do not turn Neo itself into Sergeant; use Sergeant as donor and engineering/review infrastructure.

## TTG Device X-Ray

Borrow/adapt:

- probe → normalize → correlate → fingerprint → challenge → certify → plan → evidence;
- identity correlation;
- read-first evidence discipline.

Neo owns writes/installations.

## Oracle

Borrow/adapt:

- trusted local bridge concepts;
- explicit UAC/elevation authority boundaries;
- structured requests/results;
- reconnect/resume thinking.

## Nancies-Readverse

Borrow/adapt:

- candidate-source/fallback/verification patterns;
- grounded evidence before claiming success.

## WinUtil (`jaydumisuni/winutil`)

Borrow/adapt:

- breadth of tweak/debloat knowledge;
- data-driven tweak/application catalogues;
- reversible apply/undo concepts;
- presets;
- headless/terminal operation;
- Windows feature/repair ideas;
- Winget/Chocolatey package identifiers where useful.

Do **not** copy arbitrary PowerShell-script execution as Neo's core architecture. Re-engineer applicable actions into typed Rust transactions wherever possible.

---

# 25. GUI and CLI are one engine

Do not implement independent GUI and CLI business logic.

Architecture:

```text
Neo GUI ─┐
         ├── Neo Core
Neo CLI ─┘
```

No arguments to `NeoDriver.exe` should open the GUI.

Installed/path-accessible CLI should conceptually support commands such as:

- `neo scan`
- `neo status`
- `neo drivers`
- `neo drivers missing`
- `neo runtimes`
- `neo gaming`
- `neo technician`
- `neo repair`
- `neo tweaks`
- `neo debloat`
- `neo devices`
- `neo doctor`

Expert concepts:

- inspect device;
- list/rank driver candidates;
- install/rollback/export driver;
- inspect package evidence;
- apply/undo tweak;
- plan/apply profile.

CLI destructive operations must remain explicit. Batch `--yes`-style approval may exist for automation, but high-risk actions require stronger explicit risk acknowledgement.

---

# 26. Rust / Python architecture

## Production — Rust-first

Customer machine core should be Rust-based.

Planned logical modules:

- `neo-core`
- `neo-probe`
- `neo-windows`
- `neo-drivers`
- `neo-driverstore`
- `neo-matcher`
- `neo-runtimes`
- `neo-gaming`
- `neo-device-lab`
- `neo-debloat`
- `neo-tweaks`
- `neo-features`
- `neo-repair`
- `neo-tools`
- `neo-catalogue`
- `neo-policy`
- `neo-mission`
- `neo-security`
- `neo-transaction`
- `neo-rollback`
- `neo-resume`
- `neo-verify`
- `neo-history`
- `neo-download`
- `neo-cache`
- `neo-cli`

Rust owns privileged machine actions, matching, transactions, verification, concurrency, state, downloads/cache, and the installed product core.

## Engineering — Python

Python may be used for offline/development tooling such as:

- archive/release analysis;
- catalogue extraction;
- bulk INF indexing;
- manifest generation;
- duplicate detection;
- donor conversion;
- fixture generation;
- bulk catalogue validation;
- historical evidence import.

Fresh Windows with no Python installed must still run Neo normally.

---

# 27. Online and offline modes

The same core must support:

- online source;
- local verified cache;
- Technician USB/SSD pack;
- local network repository later.

## Online Neo

Small signed EXE/app; downloads only selected verified packages.

## Neo Technician Pack

Large offline package/cache repository for workshop deployment.

Possible structure:

- NeoDriver.exe
- Catalogue/
- Drivers/
- Runtimes/
- Gaming/
- DeviceLab/
- Cache/

Use content-addressed caching (e.g. SHA-256 identity) so large packages are not repeatedly downloaded.

Cache controls should include keep/delete policy, size limits, and location selection.

---

# 28. Verified operational experience

Neo may become smarter over time without machine learning.

A verified outcome may record:

- machine family;
- device/hardware identity;
- Windows build;
- package;
- precondition;
- installation result;
- postcondition;
- reboot survival;
- regression/failure evidence.

Verified evidence may strengthen future recommendations or block known-bad package/build combinations.

Raw assumptions or unverified claims must not become durable compatibility knowledge.

---

# 29. Risk classes

Suggested action risk classes:

- **LOW** — low-impact runtime or UI preference;
- **NORMAL** — signed normal PnP driver install;
- **ELEVATED** — service/network/USB-stack changes;
- **HIGH** — test-signed kernel/boot/security-relevant changes;
- **CRITICAL / MANUAL ONLY** — firmware/BIOS and similarly consequential operations.

Confirmation depth scales with risk.

---

# 30. User-facing recommendation controls

Every appropriate item may expose:

- install;
- update;
- repair;
- reinstall;
- rollback;
- ignore;
- keep/pin current version;
- never recommend this version;
- show details;
- compare candidates.

Pinned/ignored decisions should be remembered and shown transparently rather than repeatedly nagging the user.

---

# 31. Confidence/explanation

Neo should expose why a recommendation exists.

Example evidence dimensions:

- exact hardware match;
- exact SUBSYS match;
- OS/build support;
- architecture;
- valid signature/catalogue;
- OEM match;
- known conflict state;
- verified known-good history.

Lower-confidence recommendations must surface uncertainty and may require manual confirmation rather than automatic preselection.

---

# 32. THETECHGUY / Sergeant engineering execution doctrine

Neo implementation must use the THETECHGUY engineering cycle:

1. Understand / recover evidence
2. Build
3. Review
4. Freeze
5. Prove
6. Submit / Ship

Tests are proof, not the primary discovery mechanism. Engineering should already justify why execution is expected to work.

## 32.1 Recursive 20/2 rule

Sergeant's documented tenfold method is mandatory for Neo implementation work.

Work equivalent to two normal workers gets a minimum formation of **20 distinct private/evidence lanes**. Larger justified work scales upward.

The multiplier is for coverage, not duplicate noise.

For every substantial Neo task:

1. recover evidence;
2. define the bounded task and acceptance criteria;
3. decompose into distinct specialist/evidence obligations;
4. apply 20/2 parallel coverage;
5. reconcile findings through responsible officer/reviewer roles;
6. build/correct;
7. review/challenge again;
8. every discovered finding becomes a bounded child task;
9. apply 20/2 recursively to that child until closed;
10. freeze only when no named gap remains;
11. prove;
12. final gate.

No unresolved named gap can be reported as PASS.

`BLOCKED`, `DEFERRED`, `NOT CHECKED`, and `TEST PASSED` are not synonyms for `DONE`.

## 32.2 Example 20 lanes for driver matcher

Distinct lanes may cover:

1. Hardware-ID correctness
2. Compatible-ID handling
3. SUBSYS matching
4. OEM/platform matching
5. Windows-build compatibility
6. Architecture
7. INF parsing
8. CAT/signature
9. Driver ranking
10. Driver Store state
11. Generic-vs-OEM behaviour
12. Upgrade rules
13. Downgrade rules
14. Missing-driver case
15. Broken-driver case
16. USB/filter conflicts
17. Reboot requirements
18. Rollback correctness
19. GUI/CLI/core contract
20. Negative/adversarial cases

Small tasks do not require inventing twenty code changes; the formation may be twenty distinct evidence/check obligations.

## 32.3 Officer/reviewer roles

Use Sergeant-style specialties where useful:

- Engineer — architecture/correctness/contracts;
- Medic — security/elevation/privileged state;
- Mechanic — performance/concurrency/download/cache;
- Analyst — root cause/task completeness;
- Challenger — independent opposition/second opinion;
- Judge — proof/acceptance;
- Archivist — verified lessons/experience.

Sergeant remains external engineering/review infrastructure; Neo remains its own product.

---

# 33. Planned development phases

No implementation phase starts until the architecture freeze is explicitly released for execution.

0. **Architecture Freeze** — this document.
1. **Rust Core + CLI Contract** — shared mission/action/result types.
2. **System X-Ray** — read-only hardware/Windows/driver/runtime/configuration scan.
3. **Catalogue System** — packages, drivers, runtimes, tweaks, AppX manifests.
4. **Driver Intelligence** — matching/ranking/certification.
5. **Transaction Engine** — snapshot, plan, apply, verify, rollback.
6. **Controlled Driver Installation** — manual selected-driver install/repair.
7. **Runtimes & Gaming** — VC++ 2015+, DirectX legacy, .NET, Python, XNA/OpenAL/PhysX/etc.
8. **Tweaks** — typed WinUtil-derived tweak engine.
9. **Debloat** — AppX classification/removal/restore.
10. **Repair & Windows Features** — DISM/SFC/update/network/features/etc.
11. **Device Lab** — mobile/service-driver stacks.
12. **Profiles** — Fresh Windows, Gaming, Technician, Developer, Repair, Debloat, Custom.
13. **Tools** — curated optional setup/diagnostic utilities.
14. **Offline Technician Pack** — USB/SSD/cache operation.
15. **GUI Polish** — beginner/standard/expert presentation over proven engine.
16. **Packaging / Signing** — `NeoDriver.exe` and `neo` command.
17. **Whole-product 20/2 integration campaign** — cross-module review, clean proof, final release gate.

Every phase and every substantial child task follows recursive 20/2 until all named work is closed.

---

# 34. Definition of done for a Neo feature

A feature is not done because code exists or a test is green.

A feature is complete only when:

- requirement is explicit;
- real implementation matches requirement;
- beginner interaction is safe/understandable;
- standard interaction exposes rationale/control;
- expert interaction exposes required evidence/control;
- security/privilege implications are handled;
- rollback/recovery is defined where applicable;
- GUI and CLI use the same underlying contract;
- deterministic verification exists;
- named review findings are closed;
- documentation matches implementation;
- 20/2 review obligations are reconciled;
- freeze occurred before proof;
- proof confirms the frozen implementation;
- final gate passes.

---

# 35. Three-person acceptance test

Before any feature leaves planning/review, answer all three:

### Beginner
Can someone who does not understand the underlying Windows concept use it safely and understand the outcome?

### Standard user
Can someone with basic PC knowledge understand why Neo recommends the action and change the decision?

### Expert / technician
Can a knowledgeable user inspect the underlying evidence and directly control the operation when appropriate?

If any required answer is no, the feature is not fully designed.

---

# 36. Core product philosophy

Neo's canonical operating flow is:

```text
SCAN
  ↓
UNDERSTAND
  ↓
SHOW CURRENT STATE
  ↓
RECOMMEND
  ↓
USER SELECTS
  ↓
SHOW EXACT PLAN
  ↓
CAPTURE ACTUAL STATE
  ↓
APPLY
  ↓
VERIFY
  ↓
ROLL BACK / RECOVER WHEN REQUIRED
  ↓
RECORD PROVEN RESULT
```

The simplicity belongs at the surface. The engineering underneath remains evidence-driven and technician-grade.

Neo Driver is not "scan → 37 outdated drivers → update everything." It is a machine-aware Windows setup and recovery system that can serve a beginner safely, explain itself to an ordinary PC user, and expose full evidence/control to an expert without changing the underlying truth or safety model.
