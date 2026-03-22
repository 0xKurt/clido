# Feature Plan: Image and Screenshot Input

**Status:** Planned
**Target release:** V2
**Crate(s) affected:** `clido-cli`, `clido-agent`, `clido-storage`, `clido-core`
**New files:** `crates/clido-cli/src/image.rs`, `docs/guide/image-input.md`
**New dependency:** `image` crate (resize/encode), `viuer` crate (terminal rendering), `infer` crate (magic byte detection)

---

## 1. Problem Statement

Users frequently want to show the agent a screenshot of a UI bug, a rendered error dialog, a design mockup from Figma, or a hand-drawn architecture diagram. Without image input, they are forced to describe what they see in text — a lossy, time-consuming translation that often omits important visual details.

Claude Code (Electron shell) supports image paste natively because Electron provides clipboard API access. Cursor (VS Code extension) inherits VS Code's image paste support. Clido has no image input path at all: no CLI flag, no TUI slash command, no provider capability check, and no session serialization for image content blocks. A user who tries to attach an image has no affordance to do so and no error message telling them why.

This feature gap makes Clido materially less useful for frontend development, UI debugging, and design-related workflows — which are among the most common AI-assisted coding tasks.

---

## 2. Competitive Analysis

| Tool | CLI image flag | TUI paste/drag | Terminal rendering | URL as source | Resize/compress | Clear provider error |
|------|---------------|----------------|-------------------|---------------|-----------------|----------------------|
| Claude Code | No (GUI paste only) | Yes (Electron clipboard) | No (Electron renders) | No | Unknown | Partial |
| Cursor | No | Yes (VS Code) | No | No | Unknown | Partial |
| Cline (VS Code) | No | Yes (VS Code) | No | No | Unknown | No |
| Aider | Yes (`--image`) | No | No | No | No | No |
| **Clido (this plan)** | **Yes (`--image <path\|url>`)** | **Yes (`/image` command)** | **Yes (sixel/iTerm2/Kitty/ASCII)** | **Yes** | **Yes (auto, 5MB cap)** | **Yes (named model suggestion)** |

Clido's image input is the first terminal-native coding agent implementation to include terminal image rendering and URL-as-source support alongside CLI and TUI paths.

---

## 3. Our Specific Improvements

### 3.1 Multiple input paths
Clido accepts images via three paths that cover all real-world use cases:
1. `--image <path>` flag at CLI invocation for scripted or one-shot use.
2. `/image <path>` TUI slash command for attaching mid-conversation.
3. `/image <url>` for referencing images hosted on GitHub, Imgur, or internal artifact stores.

### 3.2 Terminal image rendering
When an image is attached in TUI mode, Clido renders a thumbnail inline in the chat transcript using `viuer`. Rendering priority: Kitty graphics protocol > iTerm2 inline images > sixel > `chafa`-style ASCII art fallback. This gives the user visual confirmation that the correct image was attached without leaving the terminal. No other terminal-based coding agent does this.

### 3.3 Automatic resize and compression
Images larger than 1568×1568 pixels or 5MB are automatically resized and re-encoded as JPEG before sending to the API. The resize is lossless in terms of aspect ratio (maintained exactly). The user is shown the resulting dimensions and size: `[image resized: 1568×882, 847KB]`. This prevents silent API failures due to payload size limits.

### 3.4 Provider capability validation before API call
Before attaching an image to an API request, Clido checks whether the selected model is in the vision-capable set from `pricing.toml`. If not, the error is:
```
Error: Model "claude-haiku-4-5" does not support image input.
Switch to a vision-capable model: claude-opus-4, claude-sonnet-4-5, gpt-4o.
Use: clido --model claude-opus-4
```
This prevents a confusing raw API error from the provider.

### 3.5 URL as source
`--image https://example.com/screenshot.png` fetches the image, validates it (magic bytes, size), resizes if needed, and encodes to base64 just like a local file. Redirects are followed. A 10-second timeout applies. This is useful for GitHub issue screenshots and CI artifact links.

### 3.6 Hash-based session storage
The session JSONL stores a SHA-256 hash of the image data rather than the full base64 string. This keeps session files compact. The full base64 is held in memory only during the session. Re-loading a session that contains image turns shows `[image: <hash prefix>, not available]` in the transcript — the image is not re-sent to the API on resume, but the conversation history is intact.

---

## 4. Design Decisions and Rationale

**Magic byte detection, not file extension.**
File extensions are unreliable (files renamed, no extension, wrong extension). The `infer` crate inspects the first few bytes and returns a MIME type. This prevents silently sending a corrupt or wrong-format payload to the API.

**`image` crate for resize, not ImageMagick subprocess.**
The `image` crate handles JPEG/PNG/GIF/WebP decode and encode entirely in Rust with no system dependency. ImageMagick as a subprocess would introduce a runtime dependency that is not present on many CI environments.

**`viuer` for terminal rendering, not raw sixel.**
`viuer` handles protocol negotiation automatically (Kitty, iTerm2, sixel, block characters) and degrades gracefully. Implementing this from scratch would be several hundred lines and a maintenance burden.

**Store hash in JSONL, not base64.**
A 1MB image encodes to ~1.37MB of base64. A session with 10 image turns would be 14MB+ JSONL, which breaks `less`, diff tools, and backup systems. The hash (64 hex chars) keeps sessions readable and durable. The tradeoff is that session resume cannot re-send the image — acceptable because images are used to establish context at the start of a task, not as repeated references.

**Images attached to turns, not as a separate message type.**
The Anthropic API uses content blocks within a user message. Clido models this correctly: an image-bearing user turn has `content: [{ type: "image", ... }, { type: "text", ... }]`. This is stored in session JSONL as the same structure (with base64 replaced by hash).

**`/image` as a slash command, not a special input mode.**
Slash commands are already parsed in TUI input handling. Adding `/image` is consistent with existing UX patterns (`/model`, `/session`, etc.). A special drag-and-drop mode would require terminal feature detection that is not universally supported.

---

## 5. Implementation Steps

### 5.1 New module: `crates/clido-cli/src/image.rs`

Exports:

```rust
pub struct ImageAttachment {
    pub source: ImageSource,       // File(PathBuf) | Url(String)
    pub media_type: MediaType,     // Jpeg | Png | Gif | Webp
    pub base64_data: String,       // in-memory only
    pub hash: String,              // SHA-256 hex, stored in session
    pub original_dimensions: (u32, u32),
    pub final_dimensions: (u32, u32),
    pub final_size_bytes: usize,
}

pub enum ImageSource { File(PathBuf), Url(String) }
pub enum MediaType { Jpeg, Png, Gif, Webp }

pub async fn load_image(source: ImageSource) -> Result<ImageAttachment>
pub fn detect_media_type(bytes: &[u8]) -> Result<MediaType>
pub fn resize_if_needed(bytes: Vec<u8>, media_type: MediaType) -> Result<Vec<u8>>
pub fn render_thumbnail_in_tui(attachment: &ImageAttachment)
pub fn placeholder_line(attachment: &ImageAttachment) -> String
  // returns e.g. "[image: screenshot.png 1024×768 → 1024×768, 312KB]"
```

### 5.2 CLI flag in `clido-cli/src/cli.rs`

```rust
/// Attach an image to the prompt. Accepts a file path or HTTPS URL.
/// Can be repeated to attach multiple images.
#[arg(long = "image", value_name = "PATH_OR_URL")]
pub images: Vec<String>,
```

Images are loaded before the agent loop starts, validated, and stored in `RunArgs`. They are attached to the first user turn only.

### 5.3 Provider capability check in `clido-core`

Add `supports_vision: bool` to the model metadata in `pricing.toml`:

```toml
[[models]]
id = "claude-opus-4"
supports_vision = true

[[models]]
id = "claude-haiku-4-5"
supports_vision = false
```

In `run.rs`, before starting the agent loop with image attachments, call:

```rust
fn check_vision_support(model_id: &str, models: &[ModelMeta]) -> Result<()>
```

Emit the friendly error from §3.4 if vision is not supported.

### 5.4 Content block construction in `clido-agent/src/agent_loop.rs`

Extend `UserMessage` to accept a `Vec<ContentBlock>` where `ContentBlock` is:

```rust
pub enum ContentBlock {
    Text(String),
    Image { media_type: String, base64_data: String },
}
```

When `ImageAttachment` objects are present, the first user turn's `content` is a vec of image blocks followed by the text block. Subsequent turns contain text only.

### 5.5 Session JSONL serialization in `clido-storage`

When serializing a turn that contains image blocks, replace the `base64_data` field with:

```json
{
  "type": "image_ref",
  "hash": "sha256:abc123...",
  "media_type": "image/png",
  "dimensions": [1024, 768]
}
```

When deserializing, image blocks are reconstructed as `ImageRef` (no base64 data). The agent loop must skip re-sending `ImageRef` blocks on session resume.

### 5.6 TUI changes in `tui.rs`

**`/image <path_or_url>` slash command handler:**
1. Parse the path/URL from the command args.
2. Call `image::load_image()` asynchronously.
3. On success, store the `ImageAttachment` in a `pending_images: Vec<ImageAttachment>` field on the TUI state.
4. Display the placeholder line in the input area below the cursor: `[image: filename.png 1024×768]`.
5. On next message send, attach `pending_images` to the turn and clear the vec.

**Inline rendering:**
After a message with images is sent, render the thumbnail in the transcript above the text content using `viuer::print_from_memory()` wrapped in a try block (fail silently to placeholder if rendering fails).

**Resize feedback:**
If the image was resized, show the info line: `  [image resized from 3200×2400 to 1568×1176, saved as JPEG]`.

### 5.7 Update `SLASH_COMMANDS` constant in `tui.rs`

Add:
```rust
("/image", "Attach an image to the next message. Usage: /image <path|url>"),
```

---

## 6. Config Schema

No new `[images]` config section is needed for V2. Behavior is controlled by CLI flags and slash commands. Future config candidates (for V3):

```toml
[images]
# Maximum dimension (pixels) before auto-resize. Default matches Anthropic limit.
max_dimension = 1568

# Maximum size in bytes before forced JPEG re-encode. Default 5MB.
max_bytes = 5242880

# Terminal rendering protocol preference. Auto = detect from $TERM / $TERM_PROGRAM.
# Options: auto | kitty | iterm2 | sixel | ascii
render_protocol = "auto"
```

---

## 7. CLI Surface

```
clido run "What is wrong with this layout?" --image ./screenshot.png
clido run "Compare these two designs" --image ./design-v1.png --image ./design-v2.png
clido run "Fix the error shown here" --image https://github.com/user/repo/issues/error.png
```

`--image` is repeatable. Up to 5 images per turn (Anthropic API limit). Excess images emit a warning and are dropped.

---

## 8. TUI Changes Summary

- New slash command: `/image <path|url>` — attaches image to next message.
- Input area shows `[image: name.png WxH]` placeholder line when image is pending.
- Chat transcript renders image thumbnail inline using `viuer` after send.
- Resize notification line appears when auto-resize occurs.
- Unsupported provider error is shown inline in the chat, not as a process exit.
- `/help` output includes `/image` in the slash command list.

---

## 9. New `AgentEvent` Variant

No new `AgentEvent` variant is strictly required, but the `Thinking` variant can carry image context in its message string: `"Processing 2 image(s) + prompt…"`.

Optionally add:

```rust
ImageLoaded {
    filename: String,
    dimensions: (u32, u32),
    size_bytes: usize,
    resized: bool,
}
```

Rendered in TUI as a dim info line before the user message.

---

## 10. Test Plan

### Unit tests — `crates/clido-cli/src/image.rs`

**`test_magic_byte_detection_jpeg`**
Load the first 12 bytes of a JPEG fixture (`\xFF\xD8\xFF`). Assert `detect_media_type()` returns `MediaType::Jpeg`.

**`test_magic_byte_detection_png`**
Load PNG magic bytes (`\x89PNG\r\n\x1a\n`). Assert returns `MediaType::Png`.

**`test_magic_byte_detection_webp`**
Load WebP magic bytes (`RIFF????WEBP`). Assert returns `MediaType::Webp`.

**`test_magic_byte_detection_unknown_returns_err`**
Pass `b"hello world"`. Assert `detect_media_type()` returns `Err`.

**`test_resize_not_triggered_for_small_image`**
Create a 800×600 PNG in memory. Assert `resize_if_needed()` returns the original bytes unchanged and `final_dimensions == original_dimensions`.

**`test_resize_triggered_for_oversized_image`**
Create a 3200×2400 PNG in memory. Assert `resize_if_needed()` returns JPEG bytes where `final_dimensions.0 <= 1568 && final_dimensions.1 <= 1568`. Assert aspect ratio preserved within 1px rounding.

**`test_resize_triggered_for_large_file`**
Construct a PNG payload that is 6MB. Assert output is JPEG and `final_size_bytes < 5_242_880`.

**`test_placeholder_line_format`**
Given `filename = "error.png"`, `original_dimensions = (1024, 768)`, `final_dimensions = (1024, 768)`, `size_bytes = 320_000`. Assert `placeholder_line()` returns `"[image: error.png 1024×768, 312KB]"`.

**`test_placeholder_line_format_with_resize`**
Given original 3200×2400, final 1568×1176. Assert `placeholder_line()` includes `"→ 1568×1176"`.

**`test_sha256_hash_is_hex_64_chars`**
Load any image bytes. Assert `hash` field is exactly 64 hex characters.

### Unit tests — `crates/clido-core`

**`test_vision_support_check_passes_for_capable_model`**
Given model metadata with `supports_vision = true`, assert `check_vision_support()` returns `Ok(())`.

**`test_vision_support_check_fails_for_incapable_model`**
Given model metadata with `supports_vision = false`, assert `check_vision_support()` returns `Err` whose message contains the model name and lists alternatives.

### Unit tests — `crates/clido-storage`

**`test_session_serialization_stores_hash_not_base64`**
Construct a `ContentBlock::Image` with `base64_data = "abc123..."`. Serialize to JSON. Assert the JSON string does not contain `"abc123"`. Assert the JSON contains `"image_ref"` and `"hash"`.

**`test_session_deserialization_reconstructs_image_ref`**
Deserialize a JSON turn containing an `image_ref` block. Assert the resulting `ContentBlock` is `ImageRef`, not `Image` with base64 data.

### Integration tests — `crates/clido-cli/tests/image_integration.rs`

**`test_cli_image_flag_loads_and_attaches`**
Run the CLI with `--image fixtures/test.png` and a mock provider. Assert the mock provider receives a request whose first user message content contains an `image` block with `media_type = "image/png"`.

**`test_cli_image_url_fetches_and_attaches`**
Stand up a mock HTTP server serving a PNG. Run with `--image http://localhost:<port>/test.png`. Assert image block present in provider request.

**`test_cli_image_unsupported_model_emits_error`**
Run with `--image test.png --model <non-vision model>`. Assert exit code is non-zero and stderr contains the friendly error message.

---

## 11. Docs Pages

### New: `docs/guide/image-input.md`
Complete guide: attaching images via CLI (`--image`), attaching via TUI (`/image`), using URLs, supported formats, resize behavior, provider requirements, session resume behavior (images not re-sent), terminal rendering setup (Kitty/iTerm2/sixel), troubleshooting (wrong model, corrupt file, URL timeout).

### Update: `docs/guide/running-prompts.md`
Add a section "Attaching images" with the CLI example from §7.

### Update: `docs/reference/slash-commands.md`
Add `/image` to the command table with description, argument format, and example.

### Update: `docs/reference/flags.md`
Add `--image` to the flags table.

### Update: `docs/guide/tui.md`
Add a section showing the image placeholder in the input area and the inline thumbnail rendering.

---

## 12. Definition of Done

- [ ] `crates/clido-cli/src/image.rs` exists with `load_image()`, `detect_media_type()`, `resize_if_needed()`, `render_thumbnail_in_tui()`, and `placeholder_line()`.
- [ ] `--image <path>` CLI flag accepted, repeatable, loads image before agent loop starts.
- [ ] `--image <url>` fetches remote image with 10-second timeout and follows redirects.
- [ ] Magic byte detection used for all format validation (no extension reliance).
- [ ] Images larger than 1568px on either dimension are auto-resized maintaining aspect ratio.
- [ ] Images larger than 5MB are re-encoded as JPEG with quality reduction until under limit.
- [ ] Vision-capable flag present in `pricing.toml` for all models; `check_vision_support()` emits friendly error before API call if model does not support vision.
- [ ] `/image <path|url>` slash command works in TUI; image pending indicator shown in input area.
- [ ] `viuer` renders image thumbnail inline in TUI transcript; falls back to placeholder text without panic if rendering unsupported.
- [ ] Session JSONL stores `image_ref` (hash + dimensions) not base64 for image blocks.
- [ ] Session resume correctly skips re-sending image content to the API.
- [ ] Up to 5 images per turn enforced; 6th image emits warning and is dropped.
- [ ] All 15 unit and integration tests pass in CI.
- [ ] `docs/guide/image-input.md` written and linked from sidebar.
- [ ] `docs/reference/slash-commands.md` updated with `/image`.
- [ ] `docs/reference/flags.md` updated with `--image`.
