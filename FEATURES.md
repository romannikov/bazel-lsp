# Bazel LSP Feature Proposal

Based on the project review, the following standard LSP features are proposed for implementation to enhance the IDE experience.

## 1. Go to Definition (`textDocument/definition`)
**Goal**: Allow users to jump to the definition of a Bazel target from a label string.

-   **Functionality**:
    -   Detect if the cursor is on a Bazel label (e.g., `//pkg:target` or `:target`).
    -   **Local Targets** (`:name`): parsing the current file to find the rule definition.
    -   **Global Targets** (`//pkg:name` or `@repo//pkg:name`):
        -   Resolve the workspace root.
        -   Locate the specific `BUILD` or `BUILD.bazel` file in the package.
        -   Parse the target file to find the rule location.
-   **Implementation Details**:
    -   Reuse `BazelParser` to parse destination files.
    -   Implement logic to resolve labels to file paths.

## 2. Hover (`textDocument/hover`)
**Goal**: Display information about a target when hovering over its label.

-   **Functionality**:
    -   Show the **Rule Type** (e.g., `cc_library`, `java_binary`).
    -   Show the **Fully Qualified Label** (e.g., `//my/pkg:my_target`).
    -   (Optional) Show documentation strings or attributes if available.
-   **Implementation Details**:
    -   Use the same label resolution logic as "Go to Definition".
    -   Extract rule type from the parsed AST of the definition.

## 3. Document Symbols (`textDocument/documentSymbol`)
**Goal**: Provide a high-level outline of the current `BUILD` file.

-   **Functionality**:
    -   List all top-level targets (rules) in the file.
    -   Show their names and types.
    -   Allow quick navigation via the "Outline" view in the IDE.
-   **Implementation Details**:
    -   Use `parser.extract_targets` to get the list of targets.
    -   Map `BazelTarget` structs to `SymbolInformation` or `DocumentSymbol` LSP types.
