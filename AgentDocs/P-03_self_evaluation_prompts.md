# P-03 — Self-Evaluation & Verification Prompts
**Run these after every build — before you test anything yourself**
**Each prompt activates a different verification method**

Research basis: RefineBench 2025 found checklist-format self-critique
produces significantly better results than open-ended "does this look right?"
prompts. MIT Press TACL 2024 confirmed external signals (test output, linter
results) are required for reliable self-correction — prompting alone is not
sufficient. Use these prompts AND run the code.

---

## P-03-A — Structured Self-Critique (Run After Every Build)

```
ROLE:
Act as a critical code reviewer who has never seen this code before
and has no emotional investment in it being correct.

TASK:
Review the code you just wrote. Work through each checkpoint:

CHECKPOINT 1 — INTENT MATCH
State in one sentence what I asked you to build.
State in one sentence what you actually built.
Do they match exactly? If not, what's different?

CHECKPOINT 2 — LOGIC TRACE
Walk through the main execution path step by step.
At each step: could this produce a wrong result?
Under what specific conditions?

CHECKPOINT 3 — FAILURE MODES
List exactly 3 specific inputs or conditions that would cause
this code to fail, crash, or return wrong results.
Be specific — not "bad input" but "an empty string passed to
line 14 would cause a KeyError because..."

CHECKPOINT 4 — ASSUMPTION AUDIT
List every assumption you made about my project, data, file
structure, or environment that you cannot verify from what
I showed you.
Mark each: [LIKELY TRUE] or [UNVERIFIED — must confirm]

CHECKPOINT 5 — COMPLETENESS GAP
What would a complete, production-ready version of this have
that this version is missing?
Not gold-plating — the minimum needed to work reliably.

CHECKPOINT 6 — CONFIDENCE SCORE
Rate your confidence this works correctly on first run: 1–10
What specific thing would need to be true to raise that score?

Work through all six checkpoints.
Do not skip to "looks good overall."
```

---

## P-03-B — Run It and Report (External Signal Verification)

```
TASK:
Run the code you just wrote with each of these test cases.
Show me the actual output for each — not what should happen,
what does happen.

Test case 1 — Normal use:
[DESCRIBE A REALISTIC NORMAL INPUT]

Test case 2 — Empty input:
[Run with empty string / null / no input]

Test case 3 — Unexpected input:
[DESCRIBE SOMETHING UNUSUAL — very long, wrong type, special characters]

For each test case report:
- Input used
- Complete output including any errors or warnings
- Does the output match what I asked for?
- If there's an error: what is the root cause and what is the fix?

Fix every failure before telling me the code is ready.
Do not tell me what should happen — show me what does happen.
```

---

## P-03-C — Linter Pass (Static Analysis)

```
TASK:
Run a static analysis check on the code you just wrote.

For Python:
Run this analysis mentally against the code as if you were Ruff:
- Undefined names / variables used before assignment
- Unused imports
- Missing error handling on operations that can fail
- Type mismatches (e.g. string used where int expected)
- Potential None/null dereference without guard
- Any hardcoded values that should be in .env

For JavaScript / TypeScript:
Run this analysis mentally as if you were ESLint:
- Undefined variables
- Unused variables and imports  
- Missing null checks
- Any process.env access without a fallback or validation
- console.log statements left in (should be removed before sharing)

List every issue found.
For each issue: show the line, explain the problem, show the fix.
If you find nothing: explain specifically what you checked
and why you're confident each was clean.
```

---

## P-03-D — Regression Check (Did Anything Break?)

```
CONTEXT:
I just added [DESCRIBE WHAT YOU JUST BUILT] to an existing project.

ROLE:
Act as a QA tester whose job is to find regressions —
features that were working before this change and are now broken.

TASK:
Review the code change we just made.

1. IMPACT MAP
   List every existing function, route, or feature that this
   change could affect — directly (it calls them) or indirectly
   (they share data, state, or files with the changed code).

2. REGRESSION RISK
   For each item in the impact map, rate the regression risk:
   HIGH — this change directly modifies how it works
   MEDIUM — this change touches shared state or data
   LOW — this change is isolated and unlikely to affect it

3. TEST INSTRUCTIONS
   For every HIGH and MEDIUM item, give me exact steps to test
   that it still works correctly.
   Be specific: "Go to X, do Y, confirm you see Z."

4. WHAT CHANGED
   Show me a before/after summary of every line that changed.
   Confirm that nothing changed outside the intended scope.
```

---

## P-03-E — Cross-Model Review (Paste Into a Second AI)

*This prompt is designed to be pasted into a different AI tool than
the one that wrote the code. Claude reviews Gemini's work, or vice versa.*

```
CONTEXT:
A different AI coding tool wrote the following code.
I want your completely independent review.
Do not assume it is correct. Do not defer to the original author.

ROLE:
Act as a senior developer doing a pull request review on code
written by a junior developer you've never worked with before.
Your job is to find problems, not to approve.

REVIEW THESE DIMENSIONS:

1. DOES IT DO WHAT IT CLAIMS?
   Read the description below. Does the code actually implement that?
   Walk through the logic and flag any mismatch between intent and code.

2. WHAT WOULD BREAK IT?
   List specific inputs, sequences, or conditions that would cause
   wrong results, crashes, or unexpected behaviour.
   Think about: empty values, very large values, concurrent calls,
   network failures, database unavailability.

3. WHAT IS MISSING?
   What would a complete implementation include that this doesn't have?

4. WHAT WOULD YOU DO DIFFERENTLY?
   If you were writing this from scratch, what would you change and why?

5. PRIORITY FIXES
   List issues in order: must fix / should fix / nice to fix.

What this code is supposed to do:
[ONE SENTENCE]

The code:
[PASTE CODE]
```
