# Packaging instructions

Follow root and PC instructions plus Spec 0010. This area currently prepares review material only.

- `release-prerequisites.json` records ownership/defaults and pending gates. It is not runtime configuration or release approval.
- `inspect-artifact.ps1 -CandidatePath <candidate.exe>` reads hashes, signature state and version metadata. Keep this command free of launches, signing, installation and system mutation.
- `virtual-gamepad.iss` and `owned-setup.ps1` retain exact owned program/Private UDP rules, startup off, default data preservation and no forced termination/shared driver removal. Uninstall can remove only the explicitly identified current account's data; never scan other profiles.
- Confirm attribution and dependency notices before LICENSE/NOTICE. Signing credentials and official release identity remain external prerequisites.
- Validate PowerShell syntax with the parser and report actual candidate inspection separately from installer/runtime evidence.
- `check-packaging.ps1` uses mocks for process/firewall/startup cases and must never invoke real system mutation. Inno compilation requires a separately verified 6.7.3 compiler and expected SHA-256; no compiler is bundled.
- Signing helpers use existing owner-supplied credentials only. Sign the executable before packaging, then the installer, verify signer/timestamp, and hash final bytes. Android official signing fails on missing environment values. Neither signatures nor checksums confer release approval.
- The authorized provisional attribution is Virtual Gamepad contributors, recorded in project-overview.md. Refresh redistributed notices when dependencies change; generated local metadata paths never belong in notices or source control.
- ISCC PE version metadata is 0.0.0.0; use the script's exact preprocessor Ver guard plus the verified-origin compiler hash. Uninstall initialization is read only; RemoveOwned runs at usUninstall after confirmation and raises on cleanup failure before file removal.

_Drafted by /sync from the introducing change, worth a quick human pass._
