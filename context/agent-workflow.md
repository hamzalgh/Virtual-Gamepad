# Agent Workflow, Skills, and MCPs

## Purpose

The AI workflow should reduce repeated research and keep implementation aligned with the protocol/safety gates. It must not create a large toolchain that becomes another project to maintain.

The minimum effective setup is:

- Repository and nested `AGENTS.md` files for durable rules.
- One active numbered spec per implementation task.
- The Context7 MCP for current third-party library APIs.
- Official platform/upstream documentation for authoritative behavior.
- Local shell/build/test tools plus Android Studio and a physical phone.
- Focused review skills at defined gates.

No cloud/database/design MCP is required for the controller core.

## Context7 MCP

### Why it is recommended

Context7 is useful for APIs that change between library versions and are easy to misremember, including:

- Current Jetpack Compose, Lifecycle, DataStore, and AndroidX usage.
- Kotlin/coroutine APIs compatible with the pinned toolchain.
- Tokio UDP, time, shutdown, and `watch` patterns.
- Current APIs for crates selected later, such as `mdns-sd`, `tray-icon`, or Windows bindings.

It is a research assistant, not the source of truth for protocol design, Windows driver guarantees, Android permission policy, or release requirements. Official Android, Kotlin, Rust, Tokio, Microsoft, W3C, and upstream project sources win conflicts.

### Current project setup

At planning time, Context7 was not exposed in the active Codex session, so the plan was verified from official/upstream web sources instead of pretending Context7 results existed.

Recommended project-scoped Codex configuration:

```toml
# .codex/config.toml
[mcp_servers.context7]
url = "https://mcp.context7.com/mcp"
enabled = true
startup_timeout_sec = 40
```

After adding/changing project MCP configuration, start a new Codex task or restart the applicable client and confirm the Context7 tools are listed before relying on them.

The remote endpoint can be used without putting a secret in the repository, subject to its current anonymous limits. If authenticated access is needed, follow the current official Context7/Codex instructions and keep credentials in user-level configuration or environment-backed secret storage. Never commit an API key.

An official local-server alternative is:

```text
codex mcp add context7 -- npx -y @upstash/context7-mcp
```

Use the remote form first on this Windows project to avoid requiring Node/npm for every agent startup. Recheck current authentication/setup documentation if rate limits become a problem.

### Query workflow

Before coding against a third-party library:

1. Resolve the library, relevant version, and official/upstream owner in Context7.
2. Ask one narrow question tied to this code change and pinned version.
3. Ask for the lifecycle/error/cancellation behavior, not only a happy-path snippet.
4. Verify platform-sensitive claims in official docs/upstream source.
5. Record the chosen version and source in `context/library-docs.md` or the active spec.

Good query shapes:

```text
For Tokio compatible with our pinned Rust toolchain, show the current documented
pattern for a single bound UdpSocket that uses recv_from/send_to, cancellation,
and a latest-value watch channel. Include shutdown and error behavior.
```

```text
For our pinned AndroidX/Compose versions and minSdk 26/targetSdk 37, show the
current lifecycle-aware StateFlow collection and pointerInput multi-pointer APIs.
Do not use deprecated system UI or insets APIs.
```

Avoid broad “build the whole app” queries. Context7 works best when the active spec already fixes the architecture.

## MCP recommendation

### Required now

- **Context7:** current version-aware library documentation.

### Built-in/local capabilities, not extra MCP installations

- Filesystem editing/search and terminal commands.
- Git once the repository is initialized.
- Android Studio, Gradle, ADB, Logcat, emulator, Layout Inspector, and profiler.
- Rustup/Cargo, Clippy, Rustfmt, and Windows debugging/controller tools.

These are more direct and trustworthy for local builds than adding wrapper MCPs.

### Useful later

- **Browser/Chrome DevTools automation:** optional for the local Gamepad API harness and repeatable browser checks. It is not proof of arbitrary web-game compatibility.
- **GitHub connector/plugin:** optional after a remote exists, for issues, pull requests, reviews, and CI evidence. It is not needed to begin local development.

### Not needed for planned scope

- Supabase/database/storage MCPs.
- Cloud deployment/hosting MCPs.
- Figma/Canva MCPs for the fixed controller UI.
- Project-management MCPs unless the user intentionally moves the tracker out of the repository.
- Web-search plugins that duplicate official-source research without version context.

Install/connect an optional MCP only when its first owning spec begins.

## Skills recommendation

The names below refer to the currently available Codex skills. Availability may differ in another installation; the workflow does not depend on a proprietary skill to compile.

### Use at defined moments

- **`ui-ux-pro-max`:** use for Spec 0008 and later controller-layout/accessibility reviews. Its role is token/layout/interaction guidance; Android official APIs and physical touch tests remain authoritative.
- **`review-bugbot`:** run after coherent vertical slices, especially Specs 0004–0007 and before release. Fix findings only after reproducing/validating them.
- **`review-security`:** run when pairing/admission is implemented and again on the release diff.
- **`canvas`:** use when architecture, timing, state, or release evidence benefits from a visual artifact. Markdown remains the normative source.
- **`openai-docs`:** use only for current Codex/MCP/AGENTS/skills configuration questions.
- **`find-skills`:** use only when a concrete workflow gap remains, then inspect the skill before installing it.
- **`create-rule`:** use when deliberately changing repository agent rules/`AGENTS.md`, as in this planning session.

### Third-party skills

No third-party Android or Rust skill is required initially. The available marketplace search results reviewed during planning had much weaker adoption/evidence than official docs plus Context7. Adding a low-trust skill would increase instruction risk without solving a current gap.

If one is considered later:

1. Read its complete `SKILL.md` and referenced scripts.
2. Check publisher/source, maintenance, scope, and destructive/network behavior.
3. Compare its instructions with this repository and official docs.
4. Install only for a named task, not “just in case.”

### Potential repository-local skills

After two or three real slices reveal repeated steps, create small repo-local skills under `.agents/skills/` rather than encoding a giant generic workflow up front:

- `implement-spec`: open one numbered spec, verify prerequisites, implement, test, and update tracker.
- `change-protocol`: enforce doc-first change, both codecs, fixtures, and compatibility note.
- `verify-android-controller`: produce the exact Android Studio/physical-device checklist and evidence template.
- `release-candidate`: run artifact, signing, clean-install, compatibility, and known-limitations gates.

Do not create these until their repeated command/file patterns exist. A skill should automate a stable workflow, not freeze guesses.

## Task/thread workflow

### Starting a task

The prompt should name:

- The active spec path.
- The intended exit criterion.
- Which application(s) may change.
- What hardware/manual evidence is available in this task.

The agent reads root context, the active spec, the closest package instructions, current code/tests, and `context/progress-tracker.md` before editing.

### During implementation

- Keep one coherent slice in flight.
- Ask the user for manual feedback as soon as it becomes a gate, not after building dependent work.
- Use Context7 only for the APIs touched by the slice.
- Update protocol documentation before wire changes.
- Do not parallel-edit the same protocol or module from multiple agents.
- Independent review/research may run in parallel only when explicitly requested and cleanly scoped.

### Finishing a task

- Run the commands required by the closest `AGENTS.md`.
- Distinguish automated results from unperformed manual checks.
- Update the spec status/checklist and `context/progress-tracker.md`.
- State the next gate and do not silently begin a later phase.

## Review cadence

- After Spec 0002: protocol/robustness review.
- After Spec 0004: backend/unsafe/resource-cleanup review.
- After Spec 0005: safety/concurrency/fault review.
- After Spec 0007: full mapping and multi-touch review.
- After Spec 0009: security review.
- After Spec 0010: installer/privilege/secret review.
- Before Spec 0013 exit: full bug and security review on the release diff.

## Context maintenance cadence

- `progress-tracker.md`: every implementation task.
- Active spec: as acceptance evidence changes.
- `protocol-spec.md`: before any wire-format change.
- `library-docs.md`: each dependency/toolchain decision or release refresh.
- `architecture.md`: only for durable boundary/state-flow decisions.
- `ui-guidelines.md`: only for reusable UI/touch rules, not screen-specific TODOs.
- `project-overview.md`: only when product scope or priorities change.

## Official references

- [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)
- [Codex `AGENTS.md` guidance](https://learn.chatgpt.com/docs/agent-configuration/agents-md)
- [Codex skills](https://learn.chatgpt.com/docs/build-skills)
- [Context7 repository](https://github.com/upstash/context7)
- [Context7 client setup documentation](https://github.com/upstash/context7/blob/master/docs/resources/all-clients.mdx)
