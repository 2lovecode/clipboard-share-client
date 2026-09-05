# clipboard-history Specification

## Purpose

定义剪贴板共享历史列表的入队与对端同步规则，以及用户点击某条历史后仅写入本机系统剪贴板、不回传对端的行为契约。

## Requirements

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

### Requirement: Remote items appear in history
The system SHALL append received peer items to the local history list. Source labeling (local vs remote) is optional and MUST NOT be required for the history list to be considered complete.

#### Scenario: Peer push received
- **WHEN** a connected peer sends a history item
- **THEN** the item appears in the local history

### Requirement: Click writes local clipboard only
When the user selects a history item, the system SHALL write that item's payload to the local system clipboard and MUST NOT rebroadcast that selection to the peer as a consequence of the click.

#### Scenario: Click copies locally without rebroadcast
- **WHEN** the user clicks a history item
- **THEN** the corresponding payload is written to the local clipboard and no new outbound history message is sent solely because of that click

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

### Requirement: History search filtering
The system SHALL allow the user to filter the history list by a text query against item summaries, and MUST update the visible list when the query changes.

#### Scenario: Search narrows list
- **WHEN** the user enters a non-empty search query that matches a subset of history summaries
- **THEN** only matching items remain visible in the list (or the empty-state placeholder if none match)

#### Scenario: Clearing search restores list
- **WHEN** the user clears the search query
- **THEN** the full (paginated) history list is shown again

### Requirement: History pagination
The system SHALL paginate the history list with a configurable page size and controls to move between pages. Keyboard PageUp/PageDown MUST move to the previous/next page when available.

#### Scenario: Next page
- **WHEN** more than one page of history exists and the user advances to the next page
- **THEN** the UI shows the next page of items and updates the page indicator

#### Scenario: PageDown at last page
- **WHEN** the user is already on the last page and presses PageDown
- **THEN** the page does not advance past the last page

### Requirement: In-window keyboard selection
While the history surface is focused, the system SHALL support arrow-key selection of list items and Enter to apply the selected item to the local clipboard. Number keys 1–9 SHALL apply the corresponding item on the current page when present.

#### Scenario: Arrow and Enter apply selection
- **WHEN** the user highlights a history item with arrow keys and presses Enter
- **THEN** that item's payload is written to the local clipboard

#### Scenario: Number key applies current-page item
- **WHEN** the user presses digit N (1–9) and the current page has an item at that ordinal
- **THEN** that item is selected and written to the local clipboard
