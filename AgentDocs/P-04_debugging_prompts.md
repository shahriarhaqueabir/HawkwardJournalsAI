# P-04 — Debugging Prompts
**Use when something is broken**
**The right prompt for every type of failure**

---

## P-04-A — Standard Debug (Error Message + Code)

```
CONTEXT:
My [Python / JavaScript] app has a bug.

ROLE:
Act as a debugging specialist. Diagnose the root cause before
suggesting any fix. Do not patch symptoms — find why it broke.

WHAT IT SHOULD DO:
[One sentence — the expected behaviour]

WHAT ACTUALLY HAPPENS:
[Exactly what you see — error, wrong output, crash, blank screen]

FULL ERROR MESSAGE (do not cut this short):
[PASTE COMPLETE ERROR INCLUDING FULL TRACEBACK]

RELEVANT CODE:
[PASTE THE FILE OR FUNCTION WHERE THE ERROR OCCURS]

DIAGNOSIS STEPS:
1. Identify the root cause — not just the symptom
2. Explain why the code broke (what assumption was wrong?)
3. Show the minimal fix
4. Confirm: does this fix the root cause or just the symptom?
5. What should I test to verify the fix is complete?

Do not change anything beyond what is needed to fix this bug.
```

---

## P-04-B — Silent Failure (No Error, Wrong Behaviour)

```
CONTEXT:
My code runs without errors but produces wrong results.

ROLE:
Act as a detective. Add instrumentation to trace the execution
so we can see exactly what's happening at each step.

WHAT I EXPECTED:
[Describe what should happen]

WHAT ACTUALLY HAPPENS:
[Describe what you observe instead]

THE CODE:
[PASTE RELEVANT FILE OR FUNCTION]

STEP 1 — INSTRUMENT:
Add detailed print / console.log statements throughout this code
so I can trace the exact value of every variable at every step.
Place them at: function entry, after each transformation,
before each return, and inside every conditional branch.

I will run the instrumented version and paste the output back to you.
Do not suggest a fix yet — instrument first.

---

[After running and pasting output back, use this continuation:]

Here is the output from the instrumented code:
[PASTE FULL OUTPUT]

STEP 2 — DIAGNOSE:
Based on this output:
1. At which exact step does the value diverge from what's expected?
2. What is the root cause?
3. What is the minimal fix?
```

---

## P-04-C — Fix Introduced a New Bug

```
CONTEXT:
A previous fix worked for [ORIGINAL PROBLEM] but broke something else.

ROLE:
Act as a developer reviewing a failed patch. The goal is to fix
the original problem AND the regression together, not patch one
after another.

ORIGINAL PROBLEM:
[What you were trying to fix]

THE FIX THAT WAS APPLIED:
[PASTE THE CODE THAT WAS CHANGED]

THE NEW PROBLEM:
[What broke after the fix]

NEW ERROR OR BEHAVIOUR:
[PASTE ERROR OR DESCRIBE]

TASK:
1. Explain why the fix caused the new problem
2. Is there a root cause that makes both problems appear?
3. Write a solution that fixes both without patching them independently
4. What should I test to confirm both problems are resolved?

Do not write a separate fix for the new problem.
Find and fix the common root cause.
```

---

## P-04-D — Infinite Loop / AI Stuck in Circles

```
CONTEXT:
We've been trying to fix the same bug for several rounds and
we keep going in circles — fixing one thing breaks another.

ROLE:
Act as a senior developer doing a cold review. Ignore everything
we discussed before. Start fresh from the code and the requirement.

TASK:
Do NOT suggest another patch. Instead:

1. RESET — describe what the code is currently doing, in plain English,
   ignoring what it was supposed to do
2. DESCRIBE THE GAP — what is the difference between current behaviour
   and desired behaviour?
3. ROOT CAUSE — what is the fundamental structural reason the code
   can't produce the correct result in its current form?
4. CLEAN SOLUTION — given the root cause, what is the cleanest fix?
   Even if it means rewriting a section.

Current code:
[PASTE]

What it is supposed to do:
[ONE SENTENCE]

What it currently does:
[DESCRIBE]
```

---

## P-04-E — Clean Slate Rewrite

```
CONTEXT:
This file has been patched too many times and is now unmaintainable.
I want to rewrite it cleanly rather than continue patching.

ROLE:
Act as a developer writing this file for the first time, with full
knowledge of what the previous version was trying to do.

WHAT THIS FILE MUST DO:
[Describe the complete required behaviour]

WHAT WAS WORKING IN THE OLD VERSION (preserve this logic):
[Describe or paste the parts that worked]

WHAT FAILED IN THE OLD VERSION (do not repeat):
[Describe the recurring problems]

CONSTRAINTS:
- Write the minimum code that achieves the required behaviour
- No workarounds, no patches on top of patches
- Follow AGENTS.md conventions
- Every function has a single clear purpose
- Error handling is explicit — no silent failures

After writing:
1. Confirm the complete behaviour is covered
2. Run it with a realistic input and show me the actual output
3. Rate your confidence 1–10 and explain what would change it
```

---

## P-04-F — Common Error Quick Fixes

Save time on the most frequent errors. Paste the error, get the fix.

```
I have this error:
[PASTE ERROR]

File: [FILENAME]
Line: [LINE NUMBER IF SHOWN]
Relevant code:
[PASTE THE SPECIFIC LINES AROUND THE ERROR]

What is causing this and what is the exact fix?
```

**Common errors decoded — paste alongside the prompt above:**

| Error | Add to prompt |
|---|---|
| `ModuleNotFoundError` | "Also show me the install command and how to add it to requirements.txt" |
| `KeyError` | "Also show me how to handle this safely if the key might be missing" |
| `AttributeError: NoneType` | "Also add a None check before this line" |
| `CORS error` | "This is a browser/backend CORS issue — fix it for local development first" |
| `401 Unauthorized` | "Check how the API key is being loaded from .env and show me the fix" |
| `429 Too Many Requests` | "Add exponential backoff retry logic to this API call" |
| `Port already in use` | "Show me how to find and kill whatever is using this port on [Mac/Windows/Linux]" |
