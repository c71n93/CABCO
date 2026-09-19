# CABCO

CABCO means Code Autonomy Bounded by Component Ownership.

## Language

**Component**:
A part of a software project treated as one unit of maintenance ownership, whose implementation is separated from the rest of the project by explicitly defined interfaces.
_Avoid_: Treating component as a synonym for class or package; treating an implementation detail or an unrelated collection of files with the same ownership annotation as a component.

**Interface**:
Defines how code outside a component can use that component (its provided interface), and what the component requires from code outside it (its required interface). The interface is part of the component's contract.
_Avoid_: Restricting interface to programming-language method signatures; using interface as a synonym for the whole contract.

**Contract**:
A declaration of the externally observable behavior of a component that other components are allowed to rely on. It includes the interface and its associated behavioral requirements, such as valid inputs, outputs, errors, side effects, state changes, and other explicitly declared guarantees.
_Avoid_: Equating a contract with the mechanism used to verify it; confusing a contract change (changing the promise) with a contract violation (failing to satisfy an unchanged promise).

**Dependency**:
A relationship in which one component relies on another component to provide some part of the latter's contract.

**Maintenance ownership**:
Defines who is responsible for understanding, reviewing, and maintaining a component's internal implementation. It applies to the implementation; the contract remains under human control regardless of maintenance ownership.
_Avoid_: Ownership as authorship; ownership as authority to change the contract.

**AI-owned component**:
A component whose internal implementation AI agents may modify and maintain autonomously, provided its contract remains unchanged and satisfied. Humans are not expected to routinely understand or review that implementation.
_Avoid_: AI-written code as a synonym; code that humans cannot inspect or modify; irreversible delegation.

**Human-owned component**:
A component whose implementation humans are expected to understand and whose implementation changes require human review. AI may still generate or modify its code.
_Avoid_: Human-written code as a synonym; a prohibition on AI assistance.
