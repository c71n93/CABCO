# Declare interface boundaries explicitly

The initial prototype uses developer-declared interface boundary files rather than inferring a component's contract from source code. This supports language-independent detection of changes requiring human review, at the cost of maintaining declarations and treating any change to a declared boundary file as requiring review; declarations become policy inputs on which tools and review workflows depend.

Only the interface portion of the contract is checked in this prototype, by detecting changes to its declared files. Unchanged boundary files do not prove that the implementation satisfies the contract; general behavioral contract representation and validation remain future work.
