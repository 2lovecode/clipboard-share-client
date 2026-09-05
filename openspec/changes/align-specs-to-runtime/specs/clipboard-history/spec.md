## ADDED Requirements

### Requirement: Clipboard-change auto enqueue and sync
The system SHALL monitor the system clipboard for content changes. When a supported payload appears, the system SHALL enqueue it into local history. When a peer session is ready, the system SHALL also transmit that item to the peer. The system MUST NOT pretend an item was delivered to a peer when no session is ready.

#### Scenario: Clipboard change while connected
- **WHEN** the system clipboard content changes to a supported payload and a peer session is connected
- **THEN** the item appears in the local history and is sent to the peer

#### Scenario: Clipboard change while disconnected
- **WHEN** the system clipboard content changes to a supported payload and no peer session is connected
- **THEN** the item appears in the local history and MUST NOT be treated as delivered to a peer

#### Scenario: Oversized image rejected from sync
- **WHEN** the clipboard change is an image exceeding the implementation size limit
- **THEN** the system surfaces a clear indication and MUST NOT enqueue or transmit that oversized image as a normal sync item

### Requirement: Remote item applies to local clipboard
When a history item is received from a connected peer, the system SHALL append it to the local history and SHALL write its payload to the local system clipboard.

#### Scenario: Peer item overwrites local clipboard
- **WHEN** a connected peer sends a clipboard history item
- **THEN** the item appears in the local history and the local system clipboard contains that payload

## MODIFIED Requirements

### Requirement: Remote items appear in history
The system SHALL append received peer items to the local history list. Source labeling (local vs remote) is optional for this change and MUST NOT be required for the history list to be considered complete.

#### Scenario: Peer push received
- **WHEN** a connected peer sends a history item
- **THEN** the item appears in the local history

### Requirement: History presentation
The system SHALL present history items in a scrollable, paginated list showing at least a content summary appropriate to the payload type and relative ordering (newest identifiable). Source labels MAY be omitted until a later change adds them.

#### Scenario: Empty history placeholder
- **WHEN** the history has no items
- **THEN** the UI shows an empty-state placeholder instead of a blank failure

#### Scenario: Mixed local and remote entries
- **WHEN** both local clipboard-driven enqueues and remote receives have occurred
- **THEN** both appear in the list (source labels MAY be omitted)

#### Scenario: Newest-first identifiable order
- **WHEN** multiple history items exist
- **THEN** the list ordering makes newer items identifiable relative to older ones

## REMOVED Requirements

### Requirement: Hotkey-initiated local enqueue and push
**Reason**: Runtime uses clipboard watching for automatic enqueue and peer sync; the explicit push-hotkey model is no longer the product contract.
**Migration**: Use Clipboard-change auto enqueue and sync; keep Global hotkey quick copy in app-shell for applying existing history items locally only.
