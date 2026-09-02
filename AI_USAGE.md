# AI_USAGE.md

An honest account of how I used AI on this assignment.

## Tools and how I used them

I used OpenCode (Claude) as a design partner. Most of the AI work happened before I wrote
any code. I spent an extended session working through the payment state machine, the
concurrency mechanism, idempotency, and PSP failure handling. I ran this as a structured
Socratic session using the GSD planning framework, which captures decisions as we go and
pushes me to challenge the model instead of accepting its first answer. That is how I used
it here: I disagreed with it often, and made it defend or drop its suggestions.

A big part of the session was me asking it to research rather than reason from first
principles. When we hit the hard questions (how do you guarantee no double charge under
concurrency, what happens on a PSP timeout, how does a payment recover after the server
crashes mid-charge), I explicitly told it to stop guessing and go find how real payment
systems actually handle this. I asked it to study how Stripe and similar systems structure
async payments and crash recovery, and to bring back sourced content instead of a plausible-
sounding invention.

That research is where the core of the design came from. The sources I relied on were
Stripe's idempotency docs and engineering blog, Stripe's PaymentIntent lifecycle and webhook
docs, and Brandur Leach's write-ups on idempotency keys as recovery records and the
transactionally-staged job drain. From those I took the specific mechanisms in this design:
the idempotency key row as a recovery record with a `recovery_point`, atomic phases with the
foreign PSP call sitting between committed transactions, a completer/heartbeat worker that
resumes a payment stuck after a crash, a derived idempotency key sent downstream to the PSP so
a retry cannot double charge, and webhooks as the channel that delivers the eventual result.
When the model's first research pass was too vague, I sent it back to find primary sources and
to label each finding against one.

Finally, I had it read my existing Rust workspace and profile my conventions (snafu errors,
Axum with Extension-based DI, sqlx with plain SQL, and a jobs table with LISTEN/NOTIFY and a
heartbeat-recovery worker). I wanted the design to match patterns I already own and can
explain on camera, not a generic scaffold. Code for handlers and migrations will use the
same tool, reviewed line by line.

## Where I pushed back or decided against the model

The design came out of disagreement, not agreement. The main points where I overrode it:

1. **Row locks across the PSP call.** The model offered SELECT FOR UPDATE as the simple
   concurrency answer. I pushed back before it finished: holding a row lock across a 30s
   PSP call blocks the second payer on the slowest possible thing. I chose a status-
   conditional UPDATE claim instead, where the loser gets a 409 immediately and no lock is
   held across the network call.

2. **Guess vs. research.** When the model started reasoning from first principles about
   crash recovery, I stopped it and told it to search how Stripe actually solves this. That
   is where the recovery-point idempotency record and the completer worker came from, rather
   than something I invented on the spot.

3. **Always return 202.** The model proposed a hybrid: return 200 inline when the PSP is
   fast, 202 when slow. I chose to return 202 for every payment. One consistent contract is
   simpler to build, test, and explain, and it is honest about the fact that a PSP call is
   never truly synchronous. The outcome always arrives by webhook or by GET.

4. **sqlx over Diesel.** I decided the correctness-critical SQL (the conditional claim,
   FOR UPDATE SKIP LOCKED) should be explicit and auditable rather than hidden behind an ORM.
   I also reused my own worker architecture rather than any pattern the model defaulted to.

I also had to press it repeatedly on the crash-recovery questions (what happens if the
server crashes mid-processing, what happens if the PSP charged but we crashed before saving
it, what happens if the client loses the idempotency key). Its first answers were incomplete
or left the invoice stuck. I kept asking until the explanation was concrete, which is how the
completer plus derived-key design got nailed down.

## One thing the model got wrong

The model kept describing tok_timeout as a payment that eventually fails, and at one point
built a recovery story on "the timeout means the charge failed." That is wrong. The spec says
tok_timeout sleeps 30 seconds and then returns success. It is a slow success, not a failure.

This matters. If you treat the client-side timeout as a failure and reopen the invoice, you
reject a payment the PSP actually accepted, and you answer the "how does the caller find out
the eventual result" question incorrectly, because there is a real successful result to
deliver. The corrected design leaves the invoice in processing, lets the worker wait for the
real 30s success, and then fires invoice.paid. tok_network_error is the only genuinely failed
path. I confirmed the correction by re-reading the mock PSP token table in the assignment
directly instead of trusting the model's paraphrase.
