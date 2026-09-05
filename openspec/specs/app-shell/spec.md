# app-shell Specification

## Purpose

定义桌面应用壳的窗口生命周期、前后端桥接，以及与业务能力对等的主界面与配置界面可观察契约。

## Requirements

### Requirement: Desktop application shell
The system SHALL run as a native desktop application with a primary window that hosts the interactive UI. Closing or hiding the window MUST NOT by itself tear down background clipboard watching or peer session handling while the process remains running.

#### Scenario: Window hidden keeps background alive
- **WHEN** the user hides or toggles the main window closed while a peer session or clipboard watcher is active
- **THEN** background watching and the peer session continue until the process exits or the user explicitly disconnects

### Requirement: Frontend-backend bridge
The system SHALL expose a typed command/event bridge so the UI can invoke session and history actions and receive asynchronous updates (connection state changes, new history items, generated passphrase).

#### Scenario: UI receives connection state update
- **WHEN** a peer session becomes connected or disconnected
- **THEN** the UI is notified through the bridge and reflects the new state without requiring a manual refresh

#### Scenario: UI invokes history action
- **WHEN** the user selects a history item in the UI
- **THEN** the UI invokes a backend action that writes the item to the local clipboard per clipboard-history rules

### Requirement: Main history surface parity
The main UI SHALL present at least: connection status indicator, current clipboard preview, searchable history list with pagination controls, navigation to the configuration surface, and empty-state placeholder when history is empty.

#### Scenario: Main surface shows core regions
- **WHEN** the application window is visible on the history surface
- **THEN** the user can see connection status, current preview, search, paginated history, and a control to open configuration

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

### Requirement: Global hotkey window toggle
The system SHALL register a global hotkey that toggles main window visibility while the process is running.

#### Scenario: Toggle window via hotkey
- **WHEN** the user presses the configured global toggle hotkey
- **THEN** the main window visibility flips between shown and hidden

### Requirement: Global hotkey quick copy
The system SHALL register global hotkeys that copy a recent history item (by ordinal) to the local clipboard without requiring the window to be focused, and MUST NOT rebroadcast solely because of that hotkey action.

#### Scenario: Quick copy while window hidden
- **WHEN** the user presses a quick-copy hotkey for an available history ordinal
- **THEN** the corresponding item is written to the local clipboard and no outbound peer message is sent solely due to that hotkey
