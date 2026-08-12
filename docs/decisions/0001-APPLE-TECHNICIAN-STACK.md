# Neo Driver Decision 0001 — Apple Technician Stack

**Status:** APPROVED IMPLEMENTATION CLARIFICATION
**Scope:** Device Lab / Apple support
**Relationship to master plan:** This decision makes the existing `Apple Mobile Device / Recovery / DFU-related Windows drivers` requirement explicit. It does not replace the master plan and must be honored during implementation.

## Requirement

Neo Driver must provide complete technician-oriented Apple Windows driver coverage for both modern and legacy Apple Windows workflows.

Neo must detect, distinguish, install/repair where appropriate, and verify the Apple device-support stack used by:

- Apple Devices for Windows;
- iTunes for Windows;
- Apple Mobile Device support/services and USB drivers;
- iPhone/iPad/iPod normal-mode USB connectivity;
- Recovery Mode connectivity;
- DFU Mode connectivity;
- applicable Apple USB device interfaces used by technician workflows.

## Behaviour

Neo must not assume that installing iTunes is the only way to obtain or repair Apple device connectivity.

Neo should first inspect the machine and determine which Apple stack is present or appropriate:

1. Apple Devices-based environment;
2. iTunes-based environment;
3. partial/broken Apple support stack;
4. driver-only technician requirement where a full media application is unnecessary and redistribution/install policy permits a supported driver-only path.

Neo must expose the distinction clearly in Standard and Expert views while Beginner mode may present it simply as `Apple device support`.

## Detection / evidence targets

Where technically available, Neo should identify and correlate:

- installed Apple Devices application state;
- installed iTunes state;
- Apple Mobile Device support components;
- Apple-related Windows services;
- installed Apple USB driver packages / INF bindings;
- connected Apple USB devices and their hardware/interface IDs;
- normal, Recovery, and DFU device states;
- current driver binding;
- driver provider/version/signer;
- problem codes;
- conflicting USB/filter stacks;
- reboot/service-restart requirements.

## Technician profile

The Neo `Technician / Phone Repair` profile must offer Apple support explicitly, including normal-device, Recovery, and DFU connectivity requirements.

The user remains in control: Apple packages/components may be recommended or preselected according to the chosen profile, but are not forced.

## Repair behaviour

For `iPhone/iPad not detected` or equivalent symptoms, Neo should diagnose before reinstalling:

- physical USB enumeration present or absent;
- device mode (normal / recovery / DFU where determinable);
- trust/connection state where relevant;
- Apple support application stack present;
- Apple Mobile Device service state where applicable;
- active Windows driver binding;
- missing/broken driver package;
- device problem code;
- USB/filter conflict;
- stale/duplicate Apple device instances where relevant.

Then Neo proposes the smallest justified repair.

## Source policy

Prefer official Apple/Microsoft-supported distribution paths when possible. Existing `Apple-windows-drivers` assets remain a Neo source vault, but every package must still pass Neo catalogue classification, hash/signature/provenance checks, compatibility gates, and licensing/redistribution policy before use.

## Verification

An installer success code alone is insufficient. Neo must re-probe after change and confirm the intended Apple device/interface can bind correctly and that the expected driver/service state is healthy.

## Anti-drift

Implementation must not reduce `Apple support` to only iTunes installation or only one USB INF. Both modern Apple Devices and legacy/current iTunes-based technician environments are part of Neo's Apple support surface.
