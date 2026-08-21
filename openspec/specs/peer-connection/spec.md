# peer-connection Specification

## Purpose

定义局域网内两台客户端如何通过 mDNS 或手动地址发现彼此、以对等方式建立会话，并用共享口令完成握手校验。

## Requirements

### Requirement: mDNS peer discovery
The system SHALL advertise a discoverable clipboard-share service on the local network via mDNS and SHALL present discovered peers (display name, address, port) to the user for selection.

#### Scenario: Peer appears in discovery list
- **WHEN** another instance on the same LAN advertises the clipboard-share service
- **THEN** the local UI lists that peer with enough information for the user to initiate a connection

#### Scenario: Discovery unavailable fallback
- **WHEN** mDNS discovery fails or returns no peers
- **THEN** the system MUST still allow connection via manual address entry

### Requirement: Manual address connection
The system SHALL allow the user to connect by entering a peer IP address and port.

#### Scenario: Connect with manual IP
- **WHEN** the user submits a valid IP and port and a matching shared passphrase
- **THEN** the system attempts a peer session to that endpoint

### Requirement: Peer-to-peer listening and dialing
Each running instance SHALL be able to accept inbound peer connections and SHALL be able to dial outbound to a discovered or manually specified peer. The system MUST prevent retaining two concurrent sessions between the same pair of instances.

#### Scenario: Inbound session accepted
- **WHEN** a remote peer dials this instance with a valid passphrase
- **THEN** a single authenticated session is established and both sides can exchange clipboard history items

#### Scenario: Simultaneous dial collision
- **WHEN** both peers dial each other at approximately the same time
- **THEN** exactly one session between them remains active

### Requirement: Shared passphrase handshake
The system SHALL require a user-configured shared passphrase during session establishment and MUST reject connections that fail passphrase verification. The system SHALL NOT claim confidentiality of payload traffic beyond this LAN anti-misconnect check.

#### Scenario: Wrong passphrase rejected
- **WHEN** a peer presents an incorrect passphrase during handshake
- **THEN** the connection is closed and no history items are exchanged

#### Scenario: Correct passphrase accepted
- **WHEN** both sides present the same configured passphrase
- **THEN** the session becomes ready for history item exchange

### Requirement: Connection status visibility
The system SHALL expose connection state to the user (at least: disconnected, connecting, connected, auth-failed, error).

#### Scenario: Connected state shown
- **WHEN** a session is successfully authenticated
- **THEN** the UI indicates connected status
