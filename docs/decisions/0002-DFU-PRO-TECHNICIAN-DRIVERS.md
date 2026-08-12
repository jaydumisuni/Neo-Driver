# Neo Driver Decision 0002 — DFU Pro Technician Drivers

**Status:** APPROVED IMPLEMENTATION CLARIFICATION
**Scope:** Device Lab / Apple / technician USB support
**Relationship to master plan:** Extends the Apple technician driver requirement. It does not replace Decision 0001 or the master plan.

## Requirement

Neo Driver must explicitly support **DFU Pro technician driver packages** in addition to the standard Apple Devices / iTunes / Apple Mobile Device / Recovery / DFU Windows stack.

DFU Pro is treated as a distinct technician package family, not assumed to be identical to Apple's standard DFU driver path.

## Evidence-first rule

Neo must not identify a package as `DFU Pro` by filename or marketing label alone. During catalogue ingestion Neo must recover and record the package's real technical identity where available:

- INF files;
- supported hardware IDs and compatible IDs;
- USB VID/PID/interface bindings;
- driver class and class GUID;
- provider;
- version/date;
- catalogue/signature;
- signer;
- installed service/filter components;
- architecture and Windows-build applicability;
- package hash/provenance;
- reboot or security-state requirements;
- conflicts with Apple, WinUSB, UsbDk, libusb, or other technician USB stacks.

## Behaviour

Neo must distinguish at least these Apple technician paths:

1. Apple Devices / standard Apple support stack;
2. iTunes / legacy-current Apple support stack;
3. normal Apple USB device mode;
4. Recovery Mode;
5. standard DFU Mode;
6. DFU Pro technician driver/package path;
7. other verified technician USB/filter stacks where separately catalogued.

Neo must not overwrite a working standard Apple binding with DFU Pro merely because DFU Pro is installed or available.

If a DFU Pro package needs a different binding or filter stack for a specific technician workflow, Neo must show the intended change, preserve the existing state, detect conflicts, request explicit approval, and provide a rollback path.

## Technician profile

`Technician / Phone Repair` must expose DFU Pro as an optional Apple technician component when a verified package is available in the Neo catalogue.

It may be recommended for workflows that actually require it, but it is never forced.

## Source policy

Existing `Apple-windows-drivers` and future approved Neo driver sources may provide DFU Pro assets, but no asset becomes installable until it passes Neo package classification, hash/signature/provenance, compatibility, conflict, and redistribution-policy gates.

## Verification

Installation success is not enough. Neo must re-probe and verify that the intended DFU/technician USB interface is bound to the expected package and that unrelated Apple normal/recovery connectivity has not been unintentionally broken.

## Anti-drift

Implementation must not collapse `DFU Pro` into a generic label for every Apple DFU device. It is a separately catalogued technician package path whose actual INF, hardware bindings, signer, dependencies, conflicts, and purpose must be recovered before installation support is certified.
