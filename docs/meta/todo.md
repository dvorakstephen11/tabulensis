# todo

- Branch 2 completion:
    - Implement code changes with GPT-5.2-Pro until it has no more major suggestions.
    - Write a summary of what has happened in this branch:
        - Did a design evaluation of the product at what felt like 50% of the way through the product development process and various AI agents found some major issues with the product. The evaluation is in unified_eval_draft.md (rename)
        - Wrote 7-branch plan to resolve those issues, which is in next_sprint_plan.md
        - I appear to have completed everything described in Branch 1.
        - Started on what was included in Branch 2
        - Ran into some major performance issues so I set up benchmarking and made some major changes in how alignments work. 
        - Experienced some major performance improvements in several of my hardest benchmarks. 
        - I appear to have completed everything described in Branch 2, but I made so many major changes in the process that I wonder how accurate much of the documentation is now.
        - I would like to consolidate, correct, and clean documentation before moving on to Branch 3 (if Branch 3 is even still needed). 
    - Documentation consolidation:
        - I have the following files that describe many of the same things:
            - excel_diff_difficulty_analysis.md
            - excel_diff_meta_programming.md
            - excel_diff_product_differentiation_plan.md
            - excel_diff_specification.md
            - excel_diff_testing_plan.md
            - next_sprint_plan.md
            - unified_grid_diff_algorithm_specification.md
        - There is probably a lot of information that is now inconsistent between the codebase and the documentation, where the codebase is superior.
        - Have GPT-5.2-Thinking-Heavy figure out what of next_sprint_plan.md is already accomplished, and what is still missing.
        - Figure out what is completely redundant between excel_diff_specification.md and unified_grid_diff_algorithm_specification.md and just remove it from whichever file is inferior
        - unified_grid_diff_algorithm_specification.md reads as a bit of a low-level implementation plan. Opus 4.5 wrote it, so I'm not entirely sure it's top-quality for something as complicated as this project. Probably need to do several rounds of reading, understanding, and paring this down, with the goal of taking the best parts, removing the worst, and making everything as concise as possible without losing useful information.
        - How useful is excel_diff_testing_plan.md now?
        - *_difficulty_analysis.md, *_meta_programming.md, and *_product_differentiation_plan.md can be kept as is, although it may be useful to consolidate difficulty analysis and product differentiation plan into a single file.
            - Either way, might be useful to see if there's anything in there that is no longer relevant.
    - Figure out what should be changed in next_sprint_plan.md 
    - Present the consolidated documentation and the new codebase to GPT-5.2-Pro and Google 3 Deepthink to get their estimate of percent completion.
        - I'll do another one after next_sprint_plan.md is complete, then I'll also do another design evaluation.



- Submit a deep research prompt about what kinds of workflows represent 80-90% of the user cases, and make sure my product is positioned to solve these problems, while postponing the solution of the rarer use cases.
- Identify weaknesses in the product document
- User analysis:
    - YouTube
        - ask Gemini 3 Pro to analyze each of the relevant YouTube videos, plus create a data set containing all the comments
        - Spreadsheet Compare, xlCompare, Synkronizer, Draftable
        - Analyze all this information for latent user needs and pain points, indexed to timestamp of the video and comment (make sure the chronology of the complaints is accounted for in the analysis)
    - Reddit
    - Reviews
        - Google reviews
    - Microsoft Support
        - Find anything that is relevant to my product
- Technical Details
    - Find papers detailing the algorithms mentioned by Gemini 3 Pro and add those files to this directory
    - Do deep research about different ways these algorithms have been applied
- Learn about all the leaders among my primary competitors
- Figure out how pbi-tools might fit into all of this
    - Can .pbix files be parsed and diffed in the same way that .xlsx files can be?
- Lots of planning on the Tauri desktop app. Haven't even started this. Compare Tauri with Electron or other cross-platform frameworks.

- Set up a system by which I can periodically compare my documentation with my entire codebase and with the latest news about my competitors, their products, and related innovations (AI advances, user behavior, security vulnerabilities, etc.) and have an ultra-powerful LLM help me update my documentation and codebase to reflect the latest information. There should also be a concise "diff" statement that I can log that will tell me what decisions and changes were made. As this log grows, I can reflect upon the decisions I made.
    - This might be something I spent $500/month to do with an API script once this is bringing in some serious money (unless I'm acquired before that happens)
- Ask how to protect my IP
- Research acquisitions of similar products in the last 5 years to estimate my acquisition potential
- Brainstorm ideas about how to expand the offering

- Spend some time thinking about the future interplay of tiny, highly-specialized models in conjunction with more capable general models. This could be a major form of income for me. I need to start curating relevant research so I can watch this sector of the industry evolve. 
- Handle password stores? Seems like a large security risk for a small value add.
- Expound upon the need to handle .xlsb and .xlsx files.
- Have multiple models (in multiple isolated threads) build models to project how LLM advancement might affect my success in MED. Start by creating an AI-enhanced prompt for this deep research. 
- Get a much better idea of the Excel workflows that intersect with Excel diff (use deep research to find user discussion about this). Figure out if there is low-hanging fruit in terms of features that are not available on the market
- Ask what the greatest weakness of Rust is with respect to this product
- Identify a possible acquisition opportunity among ALL competitors. Mix this with a deep research on product lines (with feature and compatibility comparison), pricing, and company histories. The wealth of user discussion will serve as an excellent complement to this research.
- Use LLMs to create magnificent documentation about versioning maintenance for my product, to make sure that it stays current with updates to Excel.
- What are the tradeoffs in the WASM packaging?
- Perhaps my product offering in Phase 4 isn't ambitious enough? Identify all the features competitors offer that my product won't and figure out if I should expand my scope to cover some of those gaps.
- Research international distribution options/hurdles



