## MODIFIED Requirements

### Requirement: Plain text payload
The system SHALL support capturing, transmitting, and writing plain text clipboard payloads via the clipboard-watch sync path and via history apply.

#### Scenario: Text round-trip via history click
- **WHEN** a plain text item is synced to a peer and the peer user clicks that history item
- **THEN** the peer's local clipboard contains the same plain text content

#### Scenario: Text auto-sync while connected
- **WHEN** the local clipboard changes to plain text while a peer session is connected
- **THEN** the peer receives that plain text item through the session

### Requirement: Image payload
The system SHALL support capturing, transmitting, and writing common image clipboard payloads (at least a RGBA/raw image representation suitable for clipboard write) and SHALL show an image-appropriate summary in the history list when feasible.

#### Scenario: Image push received
- **WHEN** a peer sends an image payload
- **THEN** the receiving history lists an image item and the local clipboard write path attempts to apply image data on platforms that support image clipboard write

#### Scenario: Unsupported image write on platform
- **WHEN** the user applies an image history item on a platform where image clipboard write is unavailable
- **THEN** the system surfaces a clear failure indication and MUST NOT silently pretend the write succeeded

### Requirement: Unsupported types excluded
The system MUST NOT treat arbitrary file or folder drops, or rich HTML-only clipboard formats, as supported clipboard-share payloads in this contract. Attempts to sync unsupported types MUST fail with a clear indication or be ignored without pretending success.

#### Scenario: File payload rejected
- **WHEN** the user attempts to use a file or folder as the clipboard payload for sync
- **THEN** the system does not transmit it as a supported history item and informs the user it is unsupported, or otherwise does not claim a successful sync of that file payload

## REMOVED Requirements

### Requirement: Rich text / HTML payload
**Reason**: Runtime `ClipItem` supports Text and Image only; HTML capture/write is not implemented.
**Migration**: Treat HTML-only clipboard content as unsupported unless a future change reintroduces HTML as an ADDED capability; plain text fallback remains allowed when the platform exposes plain text.
