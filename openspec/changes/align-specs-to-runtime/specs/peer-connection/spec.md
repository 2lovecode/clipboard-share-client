## ADDED Requirements

### Requirement: Noise PSK session handshake
The system SHALL establish peer sessions over an encrypted transport using a shared pre-shared key (PSK). The hosting side SHALL generate a PSK for the user to share. The joining side SHALL present the same PSK. Connections that fail PSK verification MUST be rejected and MUST NOT exchange history items.

#### Scenario: Wrong PSK rejected
- **WHEN** a peer presents an incorrect PSK during handshake
- **THEN** the connection is closed and no history items are exchanged

#### Scenario: Matching PSK accepted
- **WHEN** both sides use the same PSK during handshake
- **THEN** the session becomes ready for clipboard history item exchange

#### Scenario: Host generates shareable PSK
- **WHEN** the user starts hosting a listening session
- **THEN** the system generates a PSK and surfaces it for the user to copy and share with the joining peer

## MODIFIED Requirements

### Requirement: Manual address connection
The system SHALL allow the user to connect by entering a peer IP address and port together with the host-provided PSK. mDNS discovery is not required for a complete connection flow.

#### Scenario: Connect with manual IP
- **WHEN** the user submits a valid IP and port and a matching PSK
- **THEN** the system attempts a peer session to that endpoint

### Requirement: Peer-to-peer listening and dialing
Each running instance SHALL be able to act as a host that accepts an inbound peer connection or as a joiner that dials a manually specified peer. The system MUST NOT retain more than one active session for the local instance at a time under the Host/Join model.

#### Scenario: Inbound session accepted
- **WHEN** a remote peer dials this hosting instance with a valid PSK
- **THEN** a single authenticated session is established and both sides can exchange clipboard history items

#### Scenario: Simultaneous dial collision
- **WHEN** both peers attempt to establish sessions with each other at approximately the same time under Host/Join
- **THEN** the local instance ends with at most one active authenticated session (additional attempts fail or are replaced)

#### Scenario: Starting a new role clears prior session attempt
- **WHEN** the user starts host or join while a previous session or listen/dial attempt is active
- **THEN** the prior attempt is torn down before the new role proceeds

### Requirement: Session setup controls in configuration UI
The system SHALL expose Host and Join controls in the configuration UI, including listen port (for host), peer address and port (for join), and PSK entry for join. When hosting, the system MUST display the generated PSK so the user can share it with the peer.

#### Scenario: Generated passphrase visible when hosting
- **WHEN** the user starts hosting and the system generates a PSK (passphrase equivalent)
- **THEN** the configuration UI shows the generated PSK for copy/share

#### Scenario: Manual join fields accepted
- **WHEN** the user fills peer address, port, and PSK and submits Join
- **THEN** the system attempts connection using those values per Manual address connection rules

## REMOVED Requirements

### Requirement: mDNS peer discovery
**Reason**: Current product delivers Host/Join with manual address entry only; mDNS is not implemented and is deferred.
**Migration**: Use Manual address connection; a future change may reintroduce discovery as an additive capability.

### Requirement: Shared passphrase handshake
**Reason**: Replaced by Noise PSK session handshake with host-generated PSK and encrypted transport.
**Migration**: Follow Noise PSK session handshake requirements.
