# KooPaa 💸 — Rotational Savings on Solana

**KooPaa** is a smart contract for running *Ajo*-style rotational savings groups on the **Solana blockchain**.

Inspired by West African cooperative savings models, KooPaa enables decentralized savings groups where members contribute fixed amounts periodically, and take turns receiving payouts.

## ⚙️ What It Does

KooPaa lets users:

* 📦 **Create a savings group** with a name, contribution amount, intervals, and security deposit.
* 👥 **Join an existing group** (limited to a fixed number of participants).
* 💰 **Make periodic contributions** in USDC.
* 🪙 **Receive payouts** in turns based on group configuration.
* 🔒 **Secure deposits** ensure commitment, refundable at group closure.
* 🧾 **Track contributions, payouts, and group state** onchain.

## 🧠 Key Concepts

* **Security Deposit**: Participants deposit a fixed amount when joining to discourage dropouts.
* **Contribution Interval**: Time between required contributions (e.g. every 7 days).
* **Payout Interval**: Time between payouts to members.
* **Rounds**: Each cycle where every member contributes to and one member is paid.
* **Payout Order**: First to join, first to receive payout.

## 📦 Events

The contract emits several events for off-chain indexers and bots:

| Event                    | Description                                        |
| ------------------------ | -------------------------------------------------- |
| `AjoGroupCreatedEvent`   | Emitted when a new group is created                |
| `ParticipantJoinedEvent` | Emitted when a user joins a group                  |
| `ContributionMadeEvent`  | Emitted on each user contribution                  |
| `PayoutMadeEvent`        | Emitted when a payout is distributed               |
| `AjoGroupClosedEvent`    | Emitted when a group is closed                     |
| `RefundClaimedEvent`     | Emitted when a participant withdraws their deposit |

These events can be used to **monitor off-chain**, **notify users**, or **trigger payouts** when all contributions in a round are completed.

## 🛠️ Stack

* **Solana**: Onchain program
* **Anchor**: Framework for writing and testing Solana smart contracts
* **TypeScript SDK**: Used in the backend for parsing events and triggering logic
* **Redis (Upstash)**: Lightweight off-chain store to track progress & coordinate payouts
* **Email/Telegram**: Optional notification layer for user updates

## 🤖 Automation

KooPaa works best with an off-chain backend that:

* Tracks all emitted events
* Stores contribution state in Redis
* Sends notifications (email, Telegram, etc.)
* Automatically triggers payouts once a round is complete

> Even though the truth lives onchain, coordination (e.g. triggering payout *right when it’s ready*) is managed offchain.

## 🧪 Testing

Use [Anchor's test suite](https://book.anchor-lang.com/chapter_5/testing.html) to simulate:

* Group creation
* Participant joins
* Contribution flows
* Timed payouts
* Group closure & refunds

## 🚧 Limitations

* Time-based triggers (intervals) need to be **off-chain coordinated**
* There's no `GroupStarted` event — rounds are inferred from contributions
* Does not currently support mid-cycle joins or early exits

## 📄 License

MIT