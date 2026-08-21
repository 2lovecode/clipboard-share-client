# clipboard-history Specification

## Purpose

定义剪贴板共享历史列表的入队规则、快捷键主动推送触发条件，以及用户点击某条历史后仅写入本机系统剪贴板、不回传对端的行为契约。

## Requirements

### Requirement: Hotkey-initiated local enqueue and push
The system SHALL enqueue a local history item only when the user triggers an explicit push hotkey (or equivalent explicit UI action), and SHALL transmit that item to the connected peer when a session is ready. The system MUST NOT enqueue items solely because the system clipboard content changed.

#### Scenario: Hotkey push while connected
- **WHEN** the user triggers the push hotkey and a peer session is connected
- **THEN** the captured payload appears in the local history and is sent to the peer

#### Scenario: Clipboard change does not auto-enqueue
- **WHEN** the system clipboard content changes without an explicit push action
- **THEN** no new history item is created and nothing is sent to the peer

#### Scenario: Hotkey push while disconnected
- **WHEN** the user triggers the push hotkey and no peer session is connected
- **THEN** the system either enqueues locally only or surfaces a clear disconnected indication, and MUST NOT pretend the item was delivered to a peer

### Requirement: Remote items appear in history
The system SHALL append received peer items to the local history list with an indication that the source is remote.

#### Scenario: Peer push received
- **WHEN** a connected peer sends a history item
- **THEN** the item appears in the local history marked as remote

### Requirement: Click writes local clipboard only
When the user selects a history item, the system SHALL write that item's payload to the local system clipboard and MUST NOT rebroadcast that selection to the peer as a consequence of the click.

#### Scenario: Click copies locally without rebroadcast
- **WHEN** the user clicks a history item
- **THEN** the corresponding payload is written to the local clipboard and no new outbound history message is sent solely because of that click

### Requirement: History presentation
The system SHALL present history items in a scrollable list showing at least source (local or remote), a content summary appropriate to the payload type, and relative ordering (newest identifiable).

#### Scenario: Empty history placeholder
- **WHEN** the history has no items
- **THEN** the UI shows an empty-state placeholder instead of a blank failure

#### Scenario: Mixed local and remote entries
- **WHEN** both local pushes and remote receives have occurred
- **THEN** both appear in the list with distinguishable source labels
