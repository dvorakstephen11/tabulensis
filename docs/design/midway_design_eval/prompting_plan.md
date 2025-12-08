# First Round
    [x] Initial DT and 5.1P eval with [[design_evaluation.md]] prompt 
    [x] DT compares the 5.1P eval with its evaluation and tries to make a combined eval that incorporates all the good from both evals (DT COMB)
    [ ] 5.1P compares the DT eval with its evaluation and tries to make a combined eval that incorporates all the good from both evals (5.1P COMB)
    [ ] 5.1P looks at DT COMB and 5.1P COMB and tries to make a combined eval that incorporates all the good from both evals (5.1P COMB 2)
    [ ] DT looks at 5.1P COMB 2 and DT COMB and 5.1P COMB and figures out if 5.1P COMB 2 left anything out (UCOMB)
# Second Round
    [ ] Provide codebase and [[design_evaluation.md]] prompt to DT and 5.1P to produce the second round of independent evals
    [ ] Reiterate the process from the First Round to Produce UCOMB 2
    [ ] Have DT and 5.1P compare UCOMB and UCOMB 2 and consolidate all the best ideas into one final eval (UCOMB 3)
# Third Round
    [ ] Provide codebase, other primary documents, and UCOMB 3 to DT and 5.1P with [[preparatory_implementation_prompt.md]] to produce DT-IMP1 and 5.1P-IMP1
    [ ] Have DT compare its output with 5.1P-IMP1 and identify overlap, differences, and potential 3rd superior options
    - AT THIS POINT, THE OUTPUT THAT 5.1P and DT give are going to be too small to improve upon the plans, since it will likely truncate some of them. Therefore, I need a prompt that asks them to propose edits to the plans, while keeping everything that's good. Let's call this [[incremental_plan_improvement_prompt.md]].
# Fourth Round
    [ ] Provide codebase, other primary documents, UCOMB 3, and U-IMP1 to DT and 5.1P to produce DT-IMP2 and 5.1P-IMP2. This should be the 
    implementation for the full unified grid diff algorithm (AMR).
# Fifth Round
    [ ] Provide codebase, other primary documents, UCOMB 3, U-IMP1, and U-IMP2 to DT and 5.1P to produce DT-IMP3 and 5.1P-IMP3. This should be the implementation for the M-Query diff algorithms.
    [ ] Run one round of comparison to create U-IMP3.
# Sixth Round
    [ ] Ask DT3 and 5.1P to suggestion edits to UCOMB 3, U-IMP1, U-IMP2, and U-IMP3 to improve it.
    [ ] Have Opus 4.5 implement the edits
# Final Round
    [ ] Have Opus 4.5 look at all the final outputs and figure out if any documentation needs to change. Have it make a plan for the documentationchanges, then have a separate instance of Opus 4.5 implement the changes.
