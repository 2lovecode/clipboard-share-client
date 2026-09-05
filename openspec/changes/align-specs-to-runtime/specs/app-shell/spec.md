## MODIFIED Requirements

### Requirement: Configuration surface parity
The configuration UI SHALL allow the user to host a listening session (listen port; system-generated PSK shown after host start), join a peer (address/port + PSK), view the generated PSK when hosting, and return to the history surface. Connection outcomes MUST surface via the shared connection status contract.

#### Scenario: Host from configuration
- **WHEN** the user submits host settings with a valid listen port
- **THEN** the system begins listening for inbound peers, generates a PSK, and updates connection status accordingly

#### Scenario: Join from configuration
- **WHEN** the user submits a peer address, port, and matching PSK
- **THEN** the system attempts an outbound session and updates connection status accordingly

#### Scenario: Generated PSK shown after host start
- **WHEN** hosting has started and a PSK has been generated
- **THEN** the UI displays that PSK for the user to share with the joining peer
