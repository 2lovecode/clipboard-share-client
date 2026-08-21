# clipboard-payload Specification

## Purpose

定义对等会话中可交换的剪贴板载荷类型（纯文本、图片、富文本/HTML）及其编码、传输与写回本机剪贴板的可观察行为。

## Requirements

### Requirement: Plain text payload
The system SHALL support capturing, transmitting, and writing plain text clipboard payloads.

#### Scenario: Text round-trip via history click
- **WHEN** a plain text item is pushed to a peer and the peer user clicks that history item
- **THEN** the peer's local clipboard contains the same plain text content

### Requirement: Image payload
The system SHALL support capturing, transmitting, and writing common image clipboard payloads (at least PNG or JPEG representation) and SHALL show an image-appropriate summary or thumbnail cue in the history list when feasible.

#### Scenario: Image push received
- **WHEN** a peer sends an image payload
- **THEN** the receiving history lists an image item and clicking it writes image data to the local clipboard on platforms that support image clipboard write

#### Scenario: Unsupported image write on platform
- **WHEN** the user clicks an image history item on a platform where image clipboard write is unavailable
- **THEN** the system surfaces a clear failure indication and MUST NOT silently pretend the write succeeded

### Requirement: Rich text / HTML payload
The system SHALL support capturing, transmitting, and writing rich text represented as HTML when the platform clipboard provides or accepts HTML. If only plain text is available at capture time, the system MAY send plain text instead and MUST label the item accurately.

#### Scenario: HTML push when available
- **WHEN** the user pushes content that includes HTML rich text from the clipboard
- **THEN** the peer receives an HTML (or equivalent rich) payload and can write it back on click where the platform supports it

#### Scenario: Fallback to plain text
- **WHEN** rich HTML is not available at capture time but plain text is
- **THEN** the system sends a plain text payload rather than failing the entire push

### Requirement: Unsupported types excluded
The system MUST NOT treat arbitrary file or folder drops as supported clipboard-share payloads in this change. Attempts to push unsupported types MUST fail with a clear indication.

#### Scenario: File payload rejected
- **WHEN** the user attempts to push a file or folder as the clipboard payload
- **THEN** the system does not transmit it as a supported history item and informs the user it is unsupported

### Requirement: Payload integrity
For supported types, the receiving side MUST be able to reconstruct a payload suitable for local clipboard write without corruption of the primary content bytes (text code points or image bytes).

#### Scenario: Large image within limit
- **WHEN** an image within the implementation size limit is transmitted
- **THEN** the receiver can write the same image bytes (or lossless equivalent representation) to the clipboard on click
