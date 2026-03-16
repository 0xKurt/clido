# Idea: Skills and Workflows Registry with Native Agent Payments

## Status

Ideation. Not committed to the roadmap. This document is intentionally forward-looking.

## Summary

This document explores a product concept: a networked service layer where agents can discover, acquire, install, and execute reusable skills and workflows, with optional machine-to-machine payments using `x402` or a comparable agent-native protocol.

The core thesis is that Clido's long-term competitive position is not just as a better local coding agent, but as the runtime and trust layer for a capability economy — a system where agents acquire behavior on demand, pay for it programmatically, and execute it under policy.

The most important constraint is this:

Autonomous agent payments require wallet access. Wallet access dramatically raises the security and governance stakes. This idea is only viable if wallet access is treated as a security infrastructure problem from the beginning, not as a convenience feature added later.

## Why This Is Worth Thinking About

Today, most coding agents are isolated products:
- built-in tools only
- static system prompts
- no capability discovery or marketplace
- no programmatic monetization

That means:
- useful behaviors are duplicated across teams
- reusable workflows are hard to discover or distribute
- monetization requires manual, human-mediated flows
- agents cannot acquire new capabilities autonomously when a task requires them

If Clido had a structured registry and payment layer, it could evolve from a single-user tool into a platform — a runtime where capability is distributed, trusted, and transactable.

## Core Concepts

### Skill

A skill is a reusable capability package that teaches or extends the agent's behavior.

A skill may contain:
- instructions and playbooks
- structured metadata
- parameter definitions
- example inputs and outputs
- tool constraints or requirements
- compatibility declarations
- optional evaluation fixtures for quality validation

Examples:
- a Solidity audit playbook
- a Rust release checklist
- a test-generation pattern
- a domain-specific code style guide

Skills teach behavior. They are consumed by the agent loop, not executed as code.

### Workflow

A workflow is a more explicit, executable process.

A workflow may contain:
- ordered or graph-based steps
- declared inputs and outputs
- references to required tools or MCP servers
- subagent orchestration patterns
- retry and timeout policies
- estimated cost and duration
- trust and permission requirements
- execution mode declaration (local, remote, or hybrid)

Examples:
- "Rust crate security review"
- "Semantic refactor of API surface across a monorepo"
- "Post-merge release preparation"
- "Triage CI failures and propose fixes"

Skills teach behavior. Workflows orchestrate behavior.

### Registry

The registry is the authoritative source of available skills and workflows.

Stored information:
- package identity and version
- author and publisher identity
- descriptions, categories, and tags
- dependency declarations
- required capabilities
- pricing model and price
- trust metadata and tier
- cryptographic signatures
- reputation signals and evaluations

### Payment Layer

The payment layer enables agent-native economic behavior — the ability for software to acquire services without a human in the transaction loop.

Possible payment targets:
- paid skill downloads (one-time purchase)
- per-execution workflow fees (usage-based)
- premium hosted service invocations
- usage-based external tool access
- reputation staking or refundable deposits

`x402` is relevant here because it proposes a machine-native challenge-response flow that fits agent-to-service interaction better than a web-based checkout. Instead of redirecting a human to a payment page, the service returns a structured payment requirement and the agent fulfills it directly if its policy permits.

## The Central Idea

The bigger idea is not "sell prompts." It is:

Build a capability network where agents can discover trusted behavior, acquire it dynamically, and pay for it programmatically under user-defined policy.

That could evolve into:
- a marketplace for reusable agent capabilities
- a B2B distribution layer for expert automation
- a machine-facing alternative to app stores
- a transactional protocol for agent commerce

## Proposed System Architecture

The system has four layers.

### Layer 1: Local Clido Runtime

This is the execution core and remains the primary product.

Responsibilities:
- run the agent loop
- enforce permissions
- manage local tools and installed skills
- store sessions and entitlements
- execute workflow steps
- hold or delegate wallet authority according to policy

### Layer 2: Remote Registry / Service

This is the cloud service layer that hosts discoverable content.

Responsibilities:
- index and serve skill and workflow metadata
- handle search and discovery
- manage pricing and entitlement records
- store trust and reputation data
- expose APIs for install, purchase, verify, execute, and update

### Layer 3: Payment and Settlement Layer

Responsibilities:
- authorize and process payments
- settle transactions
- issue and verify entitlement proofs
- maintain spend records

Possible models:
- x402-compatible machine-to-machine payments
- stablecoin settlement
- prepaid account credits
- subscription entitlements
- enterprise invoicing

### Layer 4: Trust and Policy Layer

This is the most important layer and must be designed first.

Responsibilities:
- verify package cryptographic signatures
- assign and enforce trust tiers
- define execution restrictions per workflow
- govern wallet access rules
- gate sensitive workflows
- produce auditable records of all purchases and executions

## What the Service Would Store

### Skills table

Key fields:
- `skill_id`, `slug`, `name`, `summary`, `description`
- `author_id`, `version`, `license`
- `tags`, `language_targets`, `tool_requirements`, `compatibility`
- `trust_level`, `signature`
- `pricing_model`, `price`
- `rating`, `download_count`
- `created_at`, `updated_at`

### Workflows table

Key fields:
- `workflow_id`, `slug`, `name`, `summary`, `description`
- `author_id`, `version`
- `input_schema`, `output_schema`, `step_graph`
- `required_skills`, `required_tools`, `required_services`
- `estimated_cost`, `estimated_duration_seconds`
- `trust_level`, `pricing_model`, `price`
- `execution_mode` (`local` | `remote` | `hybrid`)
- `policy_metadata` (modifies files, calls external services, requires secrets, spawns subagents, etc.)

### Publishers table

Key fields:
- `publisher_id`, `display_name`, `organization`
- `verification_status`, `public_keys`
- `reputation_score`, `support_contact`, `jurisdiction`

### Entitlements table

Key fields:
- `account_id`, `agent_id`
- `resource_type`, `resource_id`
- `license_type`, `purchase_timestamp`
- `expires_at`, `usage_remaining`

### Audit and payments table

Key fields:
- `payment_id`, `payer_agent_id`, `wallet_policy_id`
- `resource_type`, `resource_id`
- `amount`, `currency`, `payment_method`
- `x402_proof`, `status`, `settlement_timestamp`

## Discovery and User Experience

The registry should be optimized for agents and developers, not for casual consumers.

### Discovery methods

- keyword and semantic search
- language- and framework-specific views
- "recommended for this repository" suggestions based on detected languages and tools
- "commonly installed together" bundles
- filter by trust tier, price, publisher, and execution mode

### Agent-side queries

An agent should be able to ask:
- "Find a workflow for Python dependency auditing."
- "Find the highest-rated free Rust release workflow."
- "Find a verified Solidity skill pack under $5."
- "Find workflows that only run locally and do not call external services."

### Human-side management

A user should be able to:
- browse and install skills manually
- inspect publisher reputation and verification status
- compare paid and free options side by side
- configure spending policy for autonomous agents
- approve or deny individual agent purchase requests
- revoke entitlements and block specific publishers

## Packaging and Distribution Models

### Model A: Cloud metadata, local execution

Flow: Clido downloads the skill or workflow package, validates the signature, stores it locally, executes locally.

Suitable for: open-source packs, text-based skills, local workflow definitions.

Trade-offs: preserves local-first behavior and privacy; harder to revoke compromised packages instantly; package signing is critical.

### Model B: Cloud metadata, remote execution

Flow: Clido discovers the workflow, pays to invoke a hosted service, sends context, receives results.

Suitable for: compute-heavy services, proprietary evaluation logic, providers who cannot or will not distribute their internals.

Trade-offs: enables premium services and easier monetization; worse privacy; creates network and trust dependencies.

### Model C: Hybrid

Flow: local skill handles orchestration; a remote step handles specialized computation or proprietary evaluation at one node.

This is likely the most practical long-term model. It lets the ecosystem support both open local content and premium remote services without forcing a hard choice.

## Payment Models

### Free and open

Installable without payment. Signed but unrestricted. Cacheable locally. This is how the ecosystem bootstraps. Premium content will not matter if there is no free content to establish baseline utility.

### One-time purchase

A skill or workflow pack purchased once and reused indefinitely. Good for curated playbooks, domain-specific workflows, and premium prompt packs.

### Usage-based purchase

A workflow or remote service charges per run, per result, or per compute unit. Good for hosted analysis services, private knowledge-backed tools, and expensive computation.

### Subscription and enterprise

Teams subscribe to a private catalog or internal marketplace. Good for company-specific workflows, regulated environments, and centrally governed usage.

## Why `x402` Fits This Problem

Traditional payment flows require a human:
- open browser
- log in
- approve a charge
- copy a token
- configure a new integration

An `x402`-style flow could reduce this to:
1. agent discovers a priced resource
2. service returns a structured payment challenge
3. agent evaluates its wallet policy
4. agent pays if policy permits
5. service grants access or executes immediately

This is much closer to how autonomous software would want to transact. The human is not removed — they define the policy — but they do not need to be present for each transaction.

## The Wallet Problem

This is the central design challenge of the entire idea.

If the agent can pay directly, it has some form of authority over funds. That authority creates real risks: misconfiguration, exploited wallets, runaway spend, and attacker-induced purchases.

The product is only viable if wallet access is treated as a security architecture problem, not a UX feature.

## Wallet Access Models

### Model 1: Fully custodial platform wallet

The service holds funds. Clido spends using an API token.

Suitable for early adoption and teams that want simplicity. High trust burden on the platform. Introduces custodial and regulatory risk.

### Model 2: User wallet delegated to Clido

Clido requests signatures or executes limited payments from the user's own wallet.

Better alignment with self-custody values. Hard UX. High risk if the wallet is broadly exposed to the agent process.

### Model 3: Session wallet / ephemeral spending wallet

The user funds a limited wallet specifically for agent sessions.

Good balance of safety and autonomy. Spend is bounded by design. Requires a wallet management flow that most users will not want.

### Model 4: Policy-gated delegated wallet (recommended medium-term)

Clido receives constrained spending authority under explicit, machine-readable rules.

Possible constraints:
- maximum spend per transaction
- maximum spend per session and per day
- allowlisted publishers and merchants only
- allowlisted workflow categories only
- require human approval above a threshold amount
- deny purchases in private or sensitive repository contexts

This model balances usability, safety, and auditability better than the alternatives.

## Recommended Trust Model for Wallet Access

Do not give Clido unrestricted wallet access.

Use layered controls that grow with demonstrated safety:

**Level 1 — No autonomous spend.** Agent discovers and proposes. Every purchase requires explicit human approval. Default for all new installations.

**Level 2 — Low-risk autonomous spend.** Agent can make small purchases within a strict policy. Example: under $1 per workflow, verified publishers only, no recurring charges, all logged.

**Level 3 — Bounded operational wallet.** Agent has a dedicated wallet with a hard daily budget. Example: $20/day, only specific workflow categories, emergency kill switch, all logged.

**Level 4 — Enterprise policy engine.** Organization defines machine spending rules centrally as policy-as-code, with approver chains, org-level budget constraints, and auditable events.

## Security Requirements for Wallet Access

If any form of autonomous wallet access is added, the following must be in place:

- explicit per-agent identity with non-transferable credentials
- key isolation: agent credentials must not share key material with user credentials
- policy-scoped spending tokens that cannot exceed declared limits
- transaction signing with audit trails
- publisher and merchant allowlists enforced before any payment
- spend velocity alerts and anomaly detection
- replay protection on payment challenges
- rate limiting on payment authorization attempts
- emergency credential revocation path for users and operators

## Publisher Trust Tiers

Not all published content should be treated equally.

### Tier 0: Unverified community content

- installable only with explicit human approval
- no autonomous purchase
- restricted execution rights by default
- no sandboxed Bash without additional confirmation

### Tier 1: Verified publisher

- identity verified by the registry operator
- packages signed with verified keys
- support contact on record
- eligible for low-risk autonomous purchase within policy

### Tier 2: Trusted partner

- passed a stronger review process
- consistent reputation history
- enterprise-friendly SLAs and guarantees
- eligible for broader automation rights

### Tier 3: First-party or internal

- highest trust
- usable in strict environments
- allowlisted by default in enterprise deployments

## Workflow Execution Safety Metadata

Every workflow should declare its operational requirements so Clido can make an informed permission decision before executing.

Required metadata fields:
- `modifies_files: bool`
- `calls_external_services: bool`
- `requires_secrets: bool`
- `requires_wallet_spend: bool`
- `spawns_subagents: bool`
- `requires_sandboxed_bash: bool`
- `operates_on_private_repos: bool`

Clido uses this metadata to:
- decide whether the workflow is permitted under current policy
- determine whether human approval is required
- determine whether wallet payment can proceed autonomously
- determine whether local-only mode blocks the workflow

## API and Protocol Sketch

Possible registry API surface:
- `GET /skills` — search and list
- `GET /skills/{id}` — fetch metadata and manifest
- `GET /workflows` — search and list
- `GET /workflows/{id}` — fetch metadata and step graph
- `POST /entitlements` — purchase or claim entitlement
- `GET /entitlements/verify` — verify existing entitlement
- `POST /execute` — invoke a hosted workflow
- `POST /ratings` — submit feedback
- `GET /publishers/{id}` — publisher profile and trust status

Agent purchase flow:
1. Search registry, find resource
2. Fetch metadata; evaluate trust tier and policy metadata
3. If priced:
   a. fetch payment challenge
   b. evaluate wallet policy
   c. if policy permits, authorize and pay
   d. receive entitlement proof
4. Install locally or invoke remotely
5. Append entitlement and audit record to local storage

## Comparison With Existing Ecosystems

Understanding where this idea differs from what already exists:

| Ecosystem | What it does | What is missing for agents |
|-----------|-------------|---------------------------|
| `npm` / `cargo` | Package distribution for code | No agent-specific metadata, no payment, no trust tiers for execution |
| VS Code Extension Marketplace | Tool distribution for IDEs | No machine-readable payment, not designed for programmatic acquisition |
| GitHub Actions Marketplace | Reusable CI workflow steps | Tightly coupled to GitHub, not a general agent capability registry |
| Cursor tools / MCP servers | Runtime tool exposure | No discovery marketplace, no payment, no trust signals |
| `x402` | Machine-native payment protocol | Not a registry; just the payment layer |

This idea combines what each of those does well and fills the gaps for autonomous agent use.

## Privacy and Data Considerations

The registry creates new data flows that must be considered:

- **Usage telemetry:** the registry can observe which skills and workflows agents install and execute. This must be disclosed and minimized.
- **Repository context leakage:** when invoking remote hosted workflows, context including code may be transmitted. This must be scoped and documented.
- **Entitlement records:** purchase history constitutes business-sensitive data that must be stored securely and access-controlled.
- **Behavioral fingerprinting:** usage patterns across many agents could identify individuals or organizations even without explicit identity disclosure.

Privacy principles for this system:
- collect only what is needed for billing and safety
- never store repository content longer than the duration of a hosted execution
- allow users to export and delete their usage history
- make telemetry opt-out possible for enterprise deployments

## Versioning and Compatibility

Skills and workflows will change over time. A versioning model is necessary.

- packages use semantic versioning
- Clido tracks the installed version and can pin to a version
- the registry retains old versions for reproducibility
- breaking changes in a workflow's `input_schema` require a major version bump
- Clido can evaluate whether an installed version satisfies the compatibility requirements of a session config

Compatibility metadata example:
```json
{
  "min_clido_version": "1.5.0",
  "required_tools": ["Read", "Bash"],
  "required_mcp_servers": []
}
```

## Economic Possibilities

### Marketplace fee

Take a platform fee on paid skill and workflow transactions.

### Managed publishing tools

Offer verification, signing, analytics, and billing infrastructure for publishers.

### Enterprise private registries

Let organizations operate private skill catalogs with internal trust policies and payment rules.

### Premium execution network

Host compute-heavy or proprietary workflows as a first-party service.

Examples:
- deep code intelligence
- specialized security scanning
- private domain-knowledge services

## Why This Could Matter Strategically

Clido as a local agent competes on model quality and UX. Both are hard to differentiate permanently.

Clido as a capability platform competes on:
- ecosystem scale and content quality
- trust mediation between publishers and agents
- transaction infrastructure for the agent economy

That second position is much harder to replicate.

## Risks

### Security

Wallet access dramatically raises the impact of any compromise. This risk is manageable but must be addressed architecturally, not as an afterthought.

### UX

Too many approval prompts will kill usability. Too few will create security incidents. The right policy defaults are a product problem, not just a technical one.

### Ecosystem quality

A registry full of low-value content stalls adoption. Curation, reputation signals, and moderation are ongoing operational commitments, not launch features.

### Trust

If users do not trust published workflows or publishers, the marketplace cannot function. Trust is earned slowly and lost quickly.

### Regulatory exposure

Custodial payments, settlement, and platform fees may create compliance obligations depending on jurisdiction, currency type, and implementation. This must be evaluated before any payment feature ships.

### Incentive misalignment

If the marketplace rewards shallow skill packs over genuinely useful workflows, ecosystem quality degrades. Rating and discovery systems must be designed to surface real quality signals.

## Recommended Product Principles

If this moves forward:

- **Keep the runtime local-first.** The registry enhances Clido; it does not replace its local execution value.
- **Separate discovery from execution.** A free registry can exist and provide value before paid execution exists.
- **Separate execution from payment.** Free skills and workflows are the foundation. Paid content must earn its place.
- **Treat wallet access as a security product.** Do not ship autonomous spending until policy, audit, and revocation systems are fully in place.
- **Start with human approval.** Autonomous spending comes after the audit trail is proven.
- **Make trust visible at every step.** Every skill and workflow should display its publisher identity, trust tier, required permissions, external service usage, and price before an agent or user acts on it.

## Recommended Rollout Path

### Phase 1: Free registry only

Deliverables:
- searchable registry with skill and workflow metadata
- installable free packages with cryptographic signatures
- trust tier metadata
- local execution only
- no payments

### Phase 2: Paid distribution with human approval

Deliverables:
- one-time purchases with explicit per-purchase approval flow
- entitlement verification
- publisher verification
- no autonomous wallet usage

### Phase 3: Hosted paid workflow execution

Deliverables:
- remote workflow invocation with usage-based billing
- audit trails for all executions
- low-risk policy controls for approved categories

### Phase 4: Bounded autonomous payments

Deliverables:
- dedicated agent wallet or policy-gated delegated spending
- configurable spend caps and allowlists
- anomaly detection and spend alerts
- enterprise policy engine

### Phase 5: Agent-native transactional economy

Deliverables:
- dynamic machine-to-machine purchasing
- workflow composition across multiple providers
- service-to-service settlement
- richer economic protocols

## How This Fits Clido Specifically

Clido already has the building blocks:
- a permission system (Phase 4.3) that can gate workflow execution
- a tool registry that can host locally-installed workflow tools
- a session storage system that can record entitlement proofs
- MCP support (Phase 8.3) that can expose purchased capabilities as tools
- a context engine that can inject skill instructions into system prompts
- a subagent architecture (Phase 4.7) that can execute workflow subgraphs

The registry and payment layer extend these rather than replacing them. Clido would use its own permission model to govern wallet access, its own session storage to record purchases, and MCP to expose remote workflow capabilities as locally-callable tools.

## Open Questions

Organized by category:

### Package format and format standards
- Should skills be plain Markdown documents, structured JSON manifests, or executable Rust packages?
- How should breaking changes in skill instructions be communicated to users and agents?
- Should a formal package schema be standardized and published?

### Execution model
- Should workflow steps execute locally, remotely, or hybrid by default?
- How should partial remote execution failures be reported and recovered?
- What is the minimum context an agent should send to a remote workflow to protect privacy?

### Payment implementation
- Is `x402` mature enough for production use today, or should the first version use simpler account credit models?
- Should wallet functionality live inside Clido, inside a companion signer process, or entirely server-side?
- How should refunds and disputes work for failed or low-quality workflow executions?

### Registry operations
- How do we prevent spam, low-value content, and malicious packages at scale?
- What reputation signals are most meaningful to agent consumers versus human developers?
- Should the registry be operated by a foundation, a company, or a decentralized protocol?

### Privacy
- How much of the registry should be public versus enterprise-private?
- What is the minimum data collection model that still enables billing and safety?

## Conclusion

This is a strong and differentiated idea. It deserves serious attention.

But it is only viable if it is built as a trust and payments infrastructure problem first, and a marketplace product second.

The registry and discovery layer are straightforward to build. The hard part is giving agents the authority to transact safely, at scale, without requiring humans to approve every purchase.

If Clido gets that trust model right, it can move from "agent application" to "agent platform with a programmable capability economy."

That is significantly bigger than a plugin system. It is the foundation for software agents that discover, evaluate, acquire, and execute capabilities on behalf of the people they serve — with full policy control and an auditable trail.
