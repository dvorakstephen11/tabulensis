Okay, I think a stopgap between the ultimate, end-state meta methodology is a long-running Codex process (probably non-interactive) that continually pulls in work for itself and then implements on that work, runs the full test and performance suite to sweep for regressions, updates all documentation, and keeps an "Executive Summary" log for me that includes many lines of the form "<YYYY-mm-dd HH:MM:SS> <Concise but clear 1-3 sentence explanation of what was done>".

The motivation behind this is the realization that GPT-5.3-Codex-XHigh is SO good at coding that any minute not spent planning or coding or testing is a minute wasted. I will routinely need to run this process from 7PM - 5AM. It needs to be maximally robust so that even if the process somehow fails early, another daemon that's always on will poll frequently, see that it stopped, and have it resume where it left off.

Here's a simple sketch
- Configurable to run for N hours
- Loops:
    - Feature/Performance Implementation Loop:
        - Planning a new feature or a performance improvement
            - EXTENSIVE phase
            - Asks all the right questions
        - Implementation
            - Includes perf cycle of pre-implementation baseline plus post-implementation comparison run
            - Full test suite should be run as well to ensure nothing regressed
            - Update all relevant documents
                - It's important that this 
        - Update executive summary
        - Evaluate remaining time allotted to the process
    - Brainstorming Loop:
        - Also VERY extensive
        - Considers the entire company of Tabulensis as a whole, knowing my financial intentions, ruminates extensively over context
            - Asks ALL the questions that are relevant to the company, then continually asks itself "What am I missing?", and "What questions have I not asked myself?" 
            - Considers the ideals in meta_methodology.md and how to approach them in wise, measured, but genuinely useful steps
        - Maps out strategic decisions and discovers important questions that I might need to offer my own opinion on, or at least be aware of.
            - Is also proactive in searching the web for itself to answer the questions as much as possible.
        - Discovers aspects of the business (e.g., user analytics, licensing strategies, etc.) that currently have no explicit documentation and creates it after 
        - Reuses existing files when that makes the most sense, otherwise it makes new documentation files in sensible locations. I want to avoid the endless multiplication of similar or redundant strategy/brainstorming/explanatory documents.
    - Cleanup Loop
        - Relatively short loop looking for overt waste, particularly in documentation. Cleans it up. Should be sparing and risk-averse. 
- Ideally I come to the computer at 5AM and I have a highly readable and informative summary of everything that transpired. It should be easy to roll back things that I don't think are useful.
- The agent should do all of its work in a series of branches outside of the main branch so that I can roll back any iterative work
- I want this process to be sufficiently modular that I could copy and paste it in other codebases that represent products that will get me to my financial goal.
