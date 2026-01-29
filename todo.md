## Three Documents
- Product Feature Strategy
    * Competitor Analysis
        * Do any products currently create a graph depicting data sources and dependencies among sets of workbooks?
        * Discover redundancies, conflicting definitions, poor logic, naming conventions.
        * Create suggested tables (this seems like a borderline AI thing)
    * Feature brainstorming
        * Can this product do sheet searches across a network and do massive discovery?
            * Don’t want to expand risk surface area too much.
            * Maybe it just makes copies of the worksheets it discovers and links to the original?
        * What are some features that NO players/products have that would be valuable?
        * Maybe Tabulensis makes it easy to provide context to AI tools?!?! Seems like a big differentiator.
            * Maybe also a way to accept the response?
                * I.e., to prompt the model in such a way that it generates predictable output that Tabulensis can use (e.g. to merge workbooks, set defaults, etc)
            * What about running performance experiments based on output from AI? That is to say, it could suggest different formulas to use and then Tabulensis could be natively set up to facilitate deterministic experimentation and comparison.
    * Near-term Improvements
        * Might as well try to improve the algorithms if I find myself having down time.
        * Need guidance about automating testing on different operating systems, or about testing on different OSs in general
        * Desktop UI
            * is it using the web viewer? It looks exactly the same as it did before that refactor.
            * I don’t like that the UI doesn’t maximize by default, or that portions of containers are too small
            * Color palette for UI? Gray, green, yellow
            * Logo? Have Claude, Gemini and GPT describe various logos, then have nano banana create it
            * Could a species named Tabulensis be useful in a logo?
    * Create a script and skill file related to the task of reducing waste
        * git checkout new branch (“waste_removal_<datestamp>” ?)
        * Codex investigation and update
        * test suite + full performance suite
        * If no regressions, commit + merge into development + push to origin
    * Make sure every run of the performance suite produces a record
    * Make a plan for finding real-world files to use to test the performance of Tabulensis
- Marketing Strategy
    * As I improve the product, what improvements map to what price points? If MVP = $80/yr, are there conceivable offerings at $300/yr or more? How far away are they?
    * Partnerships
        * Deep research about potential partnerships
        * How would I create a referal system by which people could get 25% of revenue when they give someone a link and that person buys a yearly license
        * Potential referrers:
            * A. Kelly 
            * Prowland
            * Tim Shelton
            * Josh Owens
            * Taylor Lord
            * Antone
            * Hunter Duckworth
            * David Maynard
            * Stefan Patton
            * Ethan Van Zandbergen
            * Brandon Blaylock
            * Chris Edwards
            * Spence?
            * Jeffery Vernon
        * Potential Business Clients:
            * Sourcewell
            * LBMC
            * Citizens Bank
            * Tennessee Farm Bureau
            * SWARCO
    * KRAZAM / “Sheetheads” references?
    * Ask “what’s the percentage likelihood that I get acquired for $4M? $5M?”
    * Are there any network effects I can create to raise the value of Tabulensis?
    * When should I ditch the acquisition idea and just keep profiting?
        * How does that likelihood change 12, 24, and 36 months from now?
    * Have GPT research how to test rivals' products, so I can get a better idea of how Tabulensis stacks up.
    * Create a “comparison table” comparing the Tabulensis feature set to all the other product feature sets
    * On a scale of 1 to 1,000, how marketable is my product?
    * Should I make a business account on Twitter and LinkedIn for Tabulensis?
    * Any reason I shouldn’t try a partnership w/ Sourcewell? Unlimited seats at enterprise for $5k? (per year) If 1k of these gets me $5M
        * What product mix makes this a steal?
    * What are my biggest expenses going to be over time, mapped to each main phase of the product?
    * Is it crazy to make a multi-million dollar deal with incumbents to license my product?
    * How to create demo videos?
    * Get another market analysis from GPT-5.x-Pro
    * Go down a path of creating the ultimate “get acquired” guide. Maybe even ask about trying to get acquired by an angel investor who is willing to buy my product so that I can work on the “Dialectic Graph” full-time.
    * Analyzing markets through YouTube transcripts
    * get URLs w/ openAI and Gemini
    * get transcripts w/ Gemini
    * Use the YouTube Data API to get comments (???)
    * Honestly, if I could be guaranteed $250k yearly for life (or until money is invested? *(unclear)*), I would be happy to switch to Dialectic Graph full-time
- Analytics Strategy
    * Create a plan for accepting and incorporating user feedback
        * Online voting system?
        * Telemetry and other logging and analysis
        * How to use web analytics for my product?
        * Need a system for versioning the product
        * From GPT-Pro: ```Product risks: Noisy diffs → trust collapse; Early warning: users stop using PR diffs and revert to opening Excel``` How would I know they're opening Excel?
    * What internet searches can I perform daily (via AI) to make sure I’m maximally aware of information/news that might affect my behavior?
- Other
    * Make sure last night’s prompt response helps me understand how my MVP distinguishes itself from other products on the market.
    * Update the checklist to match the current state
    * After doing all these things listed above, ask GPT-5.x-Pro how entrepreneurial it thinks I am. (Maybe rewording question?)
    * Read back through the plan that describes the stripe integration
    * What major “sections” of the product, business and strategy are not currently documented?
    * Test /fork in Codex CLI
    * What cybersecurity considerations are most relevant for this product, both from the perspective of tabulensis.com and of the program itself.
    * Once I have a truly comprehensive doc set, I can ask GPT-5.x-Pro: “What should I do today?" and “What are my greatest risks?”
    * What questions am I not asking?
    * Ask about ancillary things I can do on a daily basis to make progress
    * What factors make products and companies successful?
    * FAQs? Should I add a preemptive FAQ section? Maybe with goofy questions like "How did you make such a great piece of software?" and "Can I pay you more than $___ for Tabulensis? This is such an amazing product!" 







