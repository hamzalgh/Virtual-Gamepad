# Protocol v1 fixtures

This directory is the shared compatibility boundary for the future Android codec and the Rust codec.

`manifest.txt` uses pipe separated columns: binary file, expected codec result, message type (or base fixture for invalid cases), and named decimal fields (or mutation description). All valid fields are explicit, including zeros. Identifiers are synthetic test values, not pairing secrets. Rust tests consume every manifest row, reconstruct the typed payload from named fields, compare decoding, and require byte identical encoding. Kotlin must consume the same files in its owning spec.

`generate.ps1` is an independent reference builder using explicit offsets and a bitwise CRC implementation. It does not call the Rust codec. Run `& ./protocol/fixtures/v1/generate.ps1` from the repository root to reproduce these files, then run the Rust tests. Normal test runs never regenerate fixtures. All v1 messages are 40 bytes except deliberate invalid length cases.

The two `all_buttons` fixtures together exercise every permitted button. A single packet with all 14 bits set would be invalid because it holds opposing D-pad directions. The fixture set includes valid diagonals, extrema, both hello acknowledgement forms, both status forms, every reason registry value, wrapping sequences, reserved fields/bits, CRC corruption, ranges, and wrong lengths.

`admission.txt` describes ordered scenarios for a test handler configured with the session from `neutral.bin` and an explicit peer endpoint. Each scenario starts with no sequence baseline. `other` means a second source port. These packets remain codec valid; WrongSession, WrongSource and Stale are admission outcomes, not parser errors. Rust tests exercise this manifest without activating production pairing.

The wire format is unchanged by Spec 0002. Future format changes require the normative document, implemented codecs, and fixtures to change together.
