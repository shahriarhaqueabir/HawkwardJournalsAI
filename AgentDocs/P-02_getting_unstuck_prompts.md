# P-09 — Getting Unstuck Prompts
**Use when you've been stuck for 20+ minutes**
**Decision tree: pick the prompt that matches your situation**

---

## DECISION TREE — Which Prompt to Use

```
Is there an error message?
├── YES → Use P-09-A (Error + Code)
└── NO  → Is the output wrong?
          ├── YES → Use P-09-B (Silent Wrong Behaviour)
          └── NO  → Did it ever work?
                    ├── YES → Use P-09-C (It Worked Then Broke)
                    └── NO  → Use P-09-D (Never Worked)

Have you tried multiple fixes and keep going in circles?
└── YES → Use P-09-E (Break Out of the Loop)

Have you been stuck for 30+ minutes?
└── YES → Use P-09-F (Full Reset)
```

---

## P-09-A — I Have an Error

```
CONTEXT:
[Python / JS] app. Here is the complete error — I have not cut it short.

FULL ERROR:
[PASTE COMPLETE ERROR INCLUDING EVERY LINE OF TRACEBACK]

CODE WHERE IT HAPPENS:
[PASTE THE FILE OR THE SPECIFIC FUNCTION]

WHAT IT SHOULD DO:
[One sentence]

TASK:
1. Read the last line of the error — that is the actual problem.
   What does it mean in plain English?
2. What in the code is causing it?
3. What is the minimal fix?
4. Is this the root cause or a symptom? If a symptom, what is the cause?
5. After the fix: what should I test to confirm it's fully resolved?
```

---

## P-09-B — No Error, Wrong Output

```
CONTEXT:
My code runs without errors but produces the wrong result.
I need to trace what's happening step by step.

WHAT I EXPECTED:
[Describe expected behaviour precisely]

WHAT I ACTUALLY GET:
[Describe what you observe]

CODE:
[PASTE]

TASK — TWO STAGES:

STAGE 1:
Add print / console.log statements so I can trace:
- Every input value when a function is called
- Every variable value after each transformation
- Every conditional branch taken (print which branch)
- Every return value

Place them throughout. I will run it and paste the output.

---

[After running, paste output here and add:]

Here is the traced output:
[PASTE COMPLETE OUTPUT]

STAGE 2:
Based on this trace:
1. At which exact line does the value diverge from expected?
2. What is the root cause?
3. What is the fix?
```

---

## P-09-C — It Worked Then Broke

```
CONTEXT:
My app was working and now it isn't. Something changed.

LAST THING I DID BEFORE IT BROKE:
[Be specific: "I added a function", "I installed a package",
 "I changed an env variable", "I ran a command"]

CURRENT BEHAVIOUR:
[Error message or description of what's wrong]

TASK:
1. Based on what I changed, what is the most likely cause?
2. What else could have been affected by that change?
3. What is the minimal fix to restore the working state?
4. How do I verify the fix is complete?

If I have Git:
What command shows me exactly what changed since the last working commit?

If I don't have Git:
Based on what I described changing, what is the most likely culprit
and how do I restore it?
```

---

## P-09-D — Never Worked

```
CONTEXT:
I built this from scratch and it has never worked.
Before debugging specific lines, I need a structural review.

TASK — STRUCTURAL REVIEW FIRST:
Do not debug specific lines yet. First:

1. Read the overall structure. Is there anything fundamentally
   wrong with how this is set up?
   (Wrong file structure, missing entry point, incorrect imports,
   framework misuse, etc.)

2. Is the setup correct for [FRAMEWORK / LANGUAGE]?
   What is the expected structure for this type of app?

3. What would prevent this from ever running, regardless of
   what the individual functions do?

4. List the top 3 structural issues in priority order.

After I confirm the structure is correct, we can debug the logic.

My files:
[PASTE ALL PROJECT FILES]
```

---

## P-09-E — Break Out of the Loop

> **Note:** This prompt has been consolidated into [P-04_debugging_prompts.md](P-04_debugging_prompts.md#p-04-d---infinite-loop--ai-stuck-in-circles) as **P-04-D**.
> 
> Use [P-04-D](P-04_debugging_prompts.md#p-04-d---infinite-loop--ai-stuck-in-circles) directly — it contains the complete prompt with additional guidance.

**Quick Reference:** Use P-04-D when you've been trying multiple fixes and each one creates a new problem (going in circles).

```
See: P-04_debugging_prompts.md → P-04-D — Infinite Loop / AI Stuck in Circles
```

---

## P-09-F — Full Reset (30+ Minutes Stuck)

```
CONTEXT:
I have been stuck on this for a long time.
I need a completely fresh perspective.

ROLE:
Act as a senior developer who has never seen this problem before
and is not emotionally attached to any previous approach.

TASK:
Help me think through this from first principles.

FIVE QUESTIONS — answer all five before suggesting anything:

1. What is the actual end goal here?
   (Not the implementation — the goal. What should the user be able to do?)

2. What is the simplest possible way to achieve that goal?
   (Ignore the current approach entirely.)

3. What approach am I currently trying, and why might it be
   the wrong approach for this goal?

4. If I started over with the simplest possible approach,
   what would the code look like?

5. Is there a completely different architecture that avoids
   the problem I'm stuck on entirely?

After answering all five questions: recommend the best path forward.

What I'm trying to accomplish:
[DESCRIBE THE GOAL, NOT THE CURRENT IMPLEMENTATION]

What I've tried:
[LIST THE APPROACHES THAT FAILED]

Current broken code:
[PASTE]
```
