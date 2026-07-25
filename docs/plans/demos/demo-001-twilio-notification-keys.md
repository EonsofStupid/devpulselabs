# DEMO-001 — Get your Twilio keys for the notification service

| Field | Value |
|-------|-------|
| **Status** | `STORYBOARD — first example scenario for docs/plans/demo-guide-pipeline.md` |
| **Tier** | EXTERNAL (browser on twilio.com — Program DEMO external tier) |
| **Audience** | Layman customer — no dev vocabulary, every term explained on first use |
| **Deliverable** | One self-contained `.guide`: stills + replay clips, embed default |
| **Authored** | 2026-07-25 |

## What the customer accomplishes

By the end, the customer has handed the admin layer everything it needs
to run a **separate notification service** on their own Twilio account:
Account SID, an API Key (SID + Secret), and a Messaging Service SID
with a sending phone number in it. The admin layer manages everything
after that — the customer never touches Twilio again except billing.

Credential doctrine baked into the guide: the customer creates a
**scoped API Key** for us, NOT their master Auth Token. Revocable,
auditable, and their master credential never leaves their screen.

## The layman steps (guide content)

Each step below is one guide step. `[STILL]` = do→done frame pair with
click-highlight ring; `[REPLAY n s]` = session clip, because motion or
multi-field flow can't be carried by a frame.

1. **[REPLAY ~15s] Create your Twilio account.** Go to `twilio.com`
   and click **Sign up**. Enter your name, work email, and a password.
   *(Caption: Twilio is the phone company for software — it's what
   will actually send your text messages.)*
2. **[STILL] Verify your email.** Open the email from Twilio and click
   **Confirm your email**.
3. **[STILL] Verify your phone.** Twilio texts you a code; type it in.
   *(Caption: this proves you're a person — it is NOT the number your
   notifications will come from.)*
4. **[REPLAY ~10s] Answer the setup questions.** Pick **SMS** as the
   product and **Alerts & notifications** as what you're building —
   the choices only tailor Twilio's menus; nothing is locked in.
5. **[STILL] Meet your dashboard.** You land on the Console
   (`console.twilio.com`). Bottom-left card: **Account Info**. The
   **Account SID** (starts with `AC`) is your account's public ID —
   copy it into the admin form. *(The **Auth Token** next to it is
   your master password — leave it hidden; we never ask for it.)*
6. **[REPLAY ~20s] Create an API Key for us.** Console → **Account →
   API keys & tokens → Create API key**. Name it
   `<product>-notifications`, keep type **Standard**, click
   **Create**. Copy BOTH values into the admin form NOW: the **SID**
   (starts `SK`) and the **Secret**. *(Caption, emphasized: Twilio
   shows the Secret ONCE. If you miss it, delete the key and make a
   new one — that's normal, not a problem.)*
7. **[REPLAY ~20s] Get your sending number.** **Phone Numbers →
   Manage → Buy a number** → check **SMS** → **Search** → pick one →
   **Buy**. Trial credit covers it. *(Caption: trial accounts can
   only text numbers you've verified, and prepend a trial notice —
   upgrading with a card lifts both.)*
8. **[REPLAY ~15s] Create the Messaging Service.** **Messaging →
   Services → Create Messaging Service**, name it
   `<product>-notifications`, add your new number to its **Sender
   Pool**, finish. Copy the **Messaging Service SID** (starts `MG`)
   into the admin form. *(Caption: the service is the "from" identity
   the admin layer sends through — numbers can be swapped later
   without touching the app.)*
9. **[STILL] Hand it to the admin layer.** In the product's **Admin →
   Notifications → Twilio** form, you now have all four fields:
   Account SID (`AC…`), API Key SID (`SK…`), API Key Secret,
   Messaging Service SID (`MG…`). **Save.** Never send these by
   email or chat — only the admin form.
10. **[STILL] Prove it works.** Click **Send test notification** in
    the admin panel; the text arrives on your phone. (If it doesn't:
    Twilio Console → **Monitor → Logs → Messaging** shows every
    attempt and why it failed — the admin layer surfaces the same.)

## Capture plan

- **Driver:** browser tier — interim capture via a Claude-in-Chrome
  session on the operator's machine (operator logged into a throwaway
  /demo Twilio account), Steps recorder running OS-wide for the
  do→done stills, S1-style screen record for the replay clips. The
  proper browser-driver ADR (CDP, install-on-demand) remains its own
  pass — this run is allowed to be hand-cranked; the guide artifact is
  the point.
- **Account:** a DEMO Twilio account created for the capture — never
  the operator's real account. Keys shown in captures are revoked
  immediately after the capture session.
- **REDACTION LAW (new pipeline requirement, G-6):** step 6's Secret
  reveal and any real `AC…`/`SK…` values must be blurred/patched in
  stills and clips before export. Customer-facing `.guide`s ship ONLY
  after a redaction pass; the exporter should eventually enforce a
  redaction checklist per guide.
- **Window size:** real laptop browser size (e.g. 1366×768), never
  maximized-only — same law as app proof shots.

## Friction to watch on round 1

- Twilio's console layout shifts periodically — captions name the
  MENU PATH, stills carry the pixels; when Twilio moves things, the
  path text is what we re-verify.
- Trial-account nag banners mid-capture (dismiss before recording).
- Step 6 is the drop-off risk (Secret shown once) — the caption
  emphasis and a deliberate pause in the replay clip exist for it.
