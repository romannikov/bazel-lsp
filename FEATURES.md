# Bazel LSP Features

## Implemented Features

### 1. Go to Definition (`textDocument/definition`)
**Goal**: Allow users to jump to the definition of a Bazel target from a label string.
-   **Functionality**:
    -   Detect if the cursor is on a Bazel label (e.g., `//pkg:target` or `:target`).
    -   **Local Targets**: Jumps to the rule definition in the current file.
    -   **Global Targets**: Jumps to the `BUILD` file and rule definition in the referenced package.

### 2. Auto-Completion (`textDocument/completion`)
**Goal**: Assist in writing Bazel labels.
-   **Functionality**:
    -   Triggered by `:`.
    -   **Workspace Completion**: Suggests targets across the workspace when typing `//`.
    -   **Local Completion**: Suggests targets in the current file when typing `:`.

### 3. Code Lens (`textDocument/codeLens`)
**Goal**: Provide quick actions for runnable targets.
-   **Functionality**:
    -   presents "Build", "Run", or "Test" lenses above `cc_library`, `cc_binary`, `cc_test` and similar rules.
    -   Executes the corresponding `bazel` command in the background.

### 4. Semantic Highlighting (`textDocument/semanticTokens`)
**Goal**: Improve code readability.
-   **Functionality**:
    -   Highlights rule names (functions), attributes (keys), and string values.

### 5. Formatting (`textDocument/formatting`)
**Goal**: Keep `BUILD` files tidy.
-   **Functionality**:
    -   Sorts dependencies in `deps = [...]` lists alphabetically.

### 6. Execute Command (`workspace/executeCommand`)
**Goal**: Server-side execution of Bazel tasks.
-   **Functionality**:
    -   `bazel.build`: Builds a target.
    -   `bazel.run`: Runs a binary target.
    -   `bazel.test`: Runs a test target.

---

## Future Features

### 1. Hover (`textDocument/hover`)
**Goal**: Display information about a target when hovering over its label.
-   **Idea**: Show Rule Type and Fully Qualified Label.

### 2. Document Symbols (`textDocument/documentSymbol`)
**Goal**: Provide a high-level outline of the current `BUILD` file.
-   **Idea**: List all targets in the "Outline" view for quick navigation.

### 3. Diagnostics (`textDocument/publishDiagnostics`)
**Goal**: Show errors and warnings.
-   **Idea**: Integrate with `bazel analyze` or Starlark linter to show build errors inline.
