# Meta Methodology

Implementation plan checklist: `META_METHODOLOGY_IMPLEMENTATION_CHECKLIST.md`.

## Recurring Goals

### Until Completion

- Make use of all basic marketing avenues.
- Remove all obstacles users would face in downloading, installing, and using the product.
- Onboard all necessary vendors and their tools that are useful to my product.
- Expand automation of vendor usage.
  - Reading and synthesizing emails in Fastmail inbox.
- Figure out how to get as much overnight working done as possible
  - This will likely necessitate the creation of another OpenAI Pro account for when the limits of the original account are reached. Should probably buy that with the company card.
- Create a documentation layer that updates itself in all the necessary ways.

### Until Acquisition

- UI/UX improvement.
- Performance improvement experiment.
- New marketing efforts.
  - Could be a new demo video, a new partnership, cold sales calls, or a new ad placement.
- Market analysis and awareness, aimed toward getting acquired.
  - Includes my own reading along these lines.
- Implementing a planned feature or workflow.
- Brainstorming new features and workflows.
- Improving the internal analytical system (i.e., analyzing users' activity when using Tabulensis).
  - How much of my Fastmail, Cloudflare, Resend, and Stripe info can be read programmatically so that I can have a single dashboard I look at?
- Platform security improvements.
  - AI-driven vulnerability discovery.
- Learning how to expand my use of AI tools for every aspect of this project.
- Expand coverage of documentation for operating procedures (e.g., ...).
- Creating a log of everything I've accomplished each day and updating live checklists.
- Create the ultimate context payload for Pro. It must include... 
  - Full codebase context
  - All the SOPs, concatenated as one file
  - All relevant readings from my various vendors, concatenated as one file
  - 
- Ask Pro how I can accelerate my progress
  - Let it know that I have $30k in extra savings that I can tap into at any time, but I want to make sure that my expenditures have clear payoffs.
- Ask Pro what I can do to make this product resilient to the collapse of software value, i.e., AI coding improvements making it likely that people will build their own solutions instead of buying mine. Need actionable advice.
- Improve this meta-methodology.

## Automation Hooks

- Make a meta-doc describing this effort. Maybe create a script that will dispatch N Codex agents to perform each of these tasks (or at least some portion of the task).
  - Can I automate worktree isolation?
  - Market analysis involves copying the deep research prompt to two different deep research chats in ChatGPT, pasting the outputs into a file in the appropriate location, and having one or more Codex agents synthesize all the new information and add it to the research log (which is an input into each of the deep research chats).

## Logging / Documentation Updates

- Information about everything that is accomplished should be sorted and appended to the relevant docs, which will form context for future prompts.

## One-Time Questions

- What is my schedule for implementing these? The goal needs to be to get these all underway and finished as soon as possible each day.

## Daily Schedule

### Midnight

- An always-on script kicks off the first round of automation.
  - Codex agent that looks for performance improvement opportunities and runs an experiment.
  - Codex agent that looks for next UI/UX improvement (not sure if I want it to attempt it).
    - Must be ...
    - This could also be replaced by one or more calls to GPT-5.x-Pro.
  - Codex agent that reads meta documentation (including this file) and considers ways to improve automation, reduce friction, etc.
  - Codex agent that reads meta documentation and puts together the day's checklist of all the things that I need to do manually.

### 5am-7am

- Kickoff second round of automation (if applicable) and listen to the first set of generated audio while exercising, if applicable.
  - Deep research queries.
    - Copy the deep research prompt: `python3 scripts/deep_research_prompt.py`
    - Prompt source: `docs/meta/prompts/deep_research_market_analysis.md`
- Try to do any of the manual tasks that were decided upon in the midnight run.

### 8:20am

- Kickoff third (and probably last?) round of automation.
- Finish listening to any generated audio.
- Do some manual tasks.

### 9am-12pm

- Sourcewell and Sound BI work.

### 1pm-4:15pm/5:15pm

- Do all remaining manual tasks.
- Do any remaining Sound BI work.
