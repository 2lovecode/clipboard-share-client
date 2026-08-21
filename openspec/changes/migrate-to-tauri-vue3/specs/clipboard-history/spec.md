## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: History presentation
The system SHALL present history items in a scrollable, paginated list showing at least source (local or remote), a content summary appropriate to the payload type, and relative ordering (newest identifiable). The presentation MUST remain usable after the desktop shell migration with the same informational fields.

#### Scenario: Empty history placeholder
- **WHEN** the history has no items
- **THEN** the UI shows an empty-state placeholder instead of a blank failure

#### Scenario: Mixed local and remote entries
- **WHEN** both local pushes and remote receives have occurred
- **THEN** both appear in the list with distinguishable source labels
