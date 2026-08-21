## ADDED Requirements

### Requirement: Session setup controls in configuration UI
The system SHALL expose Host and Join controls in the configuration UI, including port (for listen), peer address and port (for dial), and shared passphrase entry. When the system generates a passphrase for hosting, it MUST display that value so the user can share it with the peer.

#### Scenario: Generated passphrase visible when hosting
- **WHEN** the user starts hosting and the system generates a passphrase
- **THEN** the configuration UI shows the generated passphrase for copy/share

#### Scenario: Manual join fields accepted
- **WHEN** the user fills peer address, port, and passphrase and submits Join
- **THEN** the system attempts connection using those values per Manual address connection rules

## MODIFIED Requirements

### Requirement: Connection status visibility
The system SHALL expose connection state to the user through the desktop shell UI (at least: disconnected, connecting, connected, auth-failed, error). Status updates MUST appear on the main surface without requiring the user to reopen the window solely to learn the new state when the window is already visible.

#### Scenario: Connected state shown
- **WHEN** a session is successfully authenticated
- **THEN** the UI indicates connected status

#### Scenario: Disconnect reason shown
- **WHEN** a session ends with a reason string
- **THEN** the UI indicates disconnected status including that reason (or an equivalent clear error indication)
