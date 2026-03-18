# P-05 — Accuracy & Checking Prompts
**Run before trusting, committing, or sharing any code**
**Multi-method verification — each catches different failures**

---

## P-05-A — Pre-Commit Full Check

```
CONTEXT:
I am about to commit this code to GitHub.

ROLE:
Act as a security-aware code reviewer doing a final gate check
before code enters version control.

TASK:
Review all changed files and check every item:

SECRETS AUDIT
- Any API keys, tokens, passwords, or secrets hardcoded anywhere?
- Any os.getenv() / process.env calls without a .env.example entry?
- Any URLs that contain credentials or tokens?

PORTABILITY AUDIT
- Any hardcoded local paths (/Users/myname/, C:\Users\, ./specific-folder/)?
- Any hardcoded localhost URLs that won't work on another machine?
- Any references to files that don't exist in the repo?

COMPLETENESS AUDIT
- Is requirements.txt / package.json up to date?
- Is .env.example complete and accurate?
- Is README.md accurate for the current state of the code?
- Does AGENTS.md reflect any new files or conventions added?
- Is CHANGELOG.md updated with what changed in this session?

QUALITY AUDIT
- Any TODO or FIXME comments left in?
- Any debug print() / console.log() statements left in?
- Any commented-out code blocks that should be deleted?

OUTPUT FORMAT:
For each issue found: [FILE] [LINE] [ISSUE] [FIX]
If nothing found in a category: state "CLEAN" explicitly.
End with: "Safe to commit: YES / NO — [reason if NO]"

Files to check:
[PASTE ALL CHANGED FILES]
```

---

## P-05-B — Pre-Share Full Review

```
CONTEXT:
I am about to share this app with friends who will run it themselves
or use it via a hosted link.

ROLE:
Act as a QA engineer doing a release gate review. Be thorough.
A user finding a bug is more costly than finding it now.

TASK — check every dimension:

FUNCTIONALITY
- Does the app do what the description says?
- Does the main feature work end-to-end?
- Does it work a second time in a row (no stale state)?
- Does it work on a fresh start (no memory of previous run)?

EDGE CASES
- What happens with empty input?
- What happens with very long input?
- What happens if a user clicks a button twice quickly?
- What happens if the AI API is slow or unavailable?
- What happens if the database is unavailable?

USER EXPERIENCE
- Is it immediately clear what the app does on first screen?
- Are all buttons, inputs, and labels self-explanatory?
- Are all error messages in plain English (no tracebacks, no codes)?
- Is there feedback when something is processing (loading state)?

SETUP FOR OTHERS
- Will a friend be able to clone this and run it without help?
- Is README.md complete with setup steps?
- Is .env.example complete with all required keys?
- Are install commands in the README?

OUTPUT:
List every issue as: [SEVERITY: BLOCKING / MAJOR / MINOR] [DESCRIPTION] [FIX]
BLOCKING = app cannot be used
MAJOR = feature broken or confusing
MINOR = polish issue, not critical

App description:
[ONE SENTENCE]

Files:
[PASTE MAIN FILES]
```

---

## P-05-C — Adversarial Test

```
CONTEXT:
I want to try to break my own app before sharing it.

ROLE:
Act as a tester whose entire job is to find failures.
You are not trying to confirm it works — you are trying to break it.

TASK:
For each attack category below, describe exactly what you would try
and what the current code would do:

EMPTY ATTACK
Every user-facing input receives: empty string, null, or zero.
Does the app crash, hang, or show a friendly error?

OVERFLOW ATTACK
Every user-facing input receives: 50,000 characters, or a
number 1,000,000x larger than expected.
Does the app crash, produce wrong results, or handle it gracefully?

TYPE ATTACK
Every input receives the wrong type: a number where text is expected,
text where a number is expected, a list where a single value is expected.
What happens?

REPEAT ATTACK
The main action is performed 20 times in a row without pause.
Does state accumulate incorrectly? Does a counter overflow?
Does a file get corrupted?

NETWORK ATTACK
The external API (Gemini/Groq/Supabase) returns a 500 error.
Then it times out after 30 seconds.
Then it returns malformed JSON.
Does the app crash or show a friendly error in each case?

STATE ATTACK
The app is left open for 2 hours then used.
A user opens two browser tabs and uses both simultaneously.
Does the app behave correctly or does shared state cause problems?

For each failure found: describe the fix.
Prioritise by likelihood in a real shared app.

Code:
[PASTE]
```

---

## P-05-D — Consistency Audit

```
CONTEXT:
I just added new code to an existing project.
I need to verify the new code is consistent with the existing style,
patterns, and conventions.

ROLE:
Act as a code reviewer whose job is to ensure stylistic and
architectural consistency across the codebase.

TASK:
Compare the new code against the existing code for consistency
on every dimension:

NAMING CONVENTIONS
Do variable names, function names, and file names follow the same
patterns? List any that differ.

ERROR HANDLING PATTERN
Does the new code handle errors the same way as existing code?
(e.g. try/except style, error message format, logging approach)

STRUCTURAL PATTERNS
Does the new code follow the same structure as similar existing code?
(e.g. if existing functions return dicts, does this too?
if existing routes use the same decorator pattern, does this too?)

SECRETS PATTERN
Does the new code load secrets the same way as existing code?

AGENTS.md COMPLIANCE
Does the new code follow every convention and rule in AGENTS.md?
List any violations.

For every inconsistency: show the existing pattern, the new pattern,
and the fix to make them consistent.

Existing code (similar functions for reference):
[PASTE 2–3 EXISTING SIMILAR FUNCTIONS]

New code to check:
[PASTE NEW CODE]

AGENTS.md conventions:
[PASTE RELEVANT SECTIONS]
```

---

## P-05-E — Information Currency Check

```
CONTEXT:
I want to verify that the technical information in this code
or these recommendations is current and accurate.

ROLE:
Act as a technical fact-checker. Be honest about the limits
of your training data.

TASK:
For each item listed below, tell me:
1. What you believe to be true
2. When your training data on this topic was likely last accurate
3. What specifically might have changed since then
4. The official URL where I can verify the current state

Items to check:
- [e.g. Gemini free tier limits: 15 req/min, 1M tokens/day]
- [e.g. Groq free tier: 14,400 req/day]
- [e.g. The model name "gemini-2.0-flash"]
- [e.g. Supabase free tier: 500MB]
- [e.g. The package "google-generativeai" install command]
- [ADD ANY SPECIFIC FACTS YOU WANT VERIFIED]

For each: mark as [LIKELY CURRENT], [MAY HAVE CHANGED], or [VERIFY BEFORE USE].

I will verify the [MAY HAVE CHANGED] and [VERIFY BEFORE USE] items
at the official sources you provide before relying on them.
```
