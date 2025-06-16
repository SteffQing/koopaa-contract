# 🐢 KooPaa Smart Contract

KooPaa is a decentralized rotating savings and credit association (ROSCA) protocol built on the Solana blockchain using the Anchor framework. It enables users to create and participate in Ajo-style savings groups, make periodic contributions, and receive scheduled payouts in a transparent and trustless manner.

---

## 🧱 Architecture Overview

KooPaa uses [Anchor](https://book.anchor-lang.com/) and [SPL Token](https://spl.solana.com/token) libraries to manage program logic, account constraints, and token transfers.

### Key Features:

* ✅ **Create savings groups (Ajo)**
* ✅ **Join with a security deposit**
* ✅ **Contribute periodically in fixed intervals**
* ✅ **Scheduled payouts to rotating participants**
* ✅ **Refunds and group closure with voting support**
* ✅ **Fully on-chain and event-emitting for dApp interop**

---

## 🛠️ Tech Stack

* **Solana** + **Anchor**
* **TypeScript** test suite with:

  * `@coral-xyz/anchor`
  * `@solana/spl-token`
  * `chai`

---

## 📁 File Structure

```
/programs/koopa/
  ├── lib.rs                // Main program logic
  ├── state/                // Account state definitions
  ├── events.rs             // On-chain event definitions
  ├── errors.rs             // Custom error types
  ├── utils.rs              // Utility functions
```

---

## 🧾 Instruction Overview

### `initialize`

Initializes the global state account with default values.

---

### `create_ajo_group`

Creates a new Ajo group with:

* Name
* Security deposit
* Contribution amount & interval
* Payout interval
* Participant count

Emits:

* `AjoGroupCreatedEvent`
* `ParticipantJoinedEvent` (for creator)

---

### `join_ajo_group`

Lets a new participant join an existing group before it starts. Transfers their security deposit to the group vault.

Emits:

* `ParticipantJoinedEvent`
* `AjoGroupStartedEvent` (once group is full)

---

### `contribute`

A participant contributes tokens based on how many rounds they've missed. Requires the group to be active and the contributor to be a member.

Emits:

* `ContributionMadeEvent`

---

### `payout`

Transfers pooled contributions to the current round's recipient if all participants have paid and it's time.

Emits:

* `PayoutMadeEvent`

---

### `close_ajo_group`

Allows participants to vote for closure. Once threshold is met, group status is marked closed and participants can withdraw refunds.

Emits:

* `AjoGroupClosedEvent`

---

### `claim_refund`

After group closure, participants can claim their unused security deposit.

Emits:

* `RefundClaimedEvent`

---

## 🔐 Accounts

* `GlobalState`: Program-wide metadata
* `AjoGroup`: A specific ROSCA group
* `TokenVault`: PDA-controlled account holding pooled tokens
* `AjoParticipant`: Embedded within each group state

---

## 🧪 Example Test Setup

```ts
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Koopa } from "../target/types/koopa";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { expect } from "chai";
```

You can write test cases for:

* Group creation
* Joining participants
* Contribution flow
* Valid payouts
* Group closure and refunds

---

## 📦 Build & Deploy

```bash
anchor build
anchor deploy
```

---

## 📜 License

MIT License.
Created by the KooPaa Team.
