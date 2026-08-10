import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";

const EXT_ID = "H5uan.filament-mat-lsp";

function fixturesDir(): string {
    const ext = vscode.extensions.getExtension(EXT_ID);
    assert.ok(ext, `Extension ${EXT_ID} should be activated`);
    return path.join(ext.extensionPath, "client", "test", "fixtures");
}

async function openDocument(relative: string): Promise<vscode.TextDocument> {
    const uri = vscode.Uri.file(path.join(fixturesDir(), relative));
    const doc = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(doc);
    await new Promise((r) => setTimeout(r, 300));
    return doc;
}

async function waitForDiagnostics(
    uri: vscode.Uri,
    predicate: (d: readonly vscode.Diagnostic[]) => boolean,
    timeoutMs = 15000
): Promise<readonly vscode.Diagnostic[]> {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
        const diags = vscode.languages.getDiagnostics(uri);
        if (predicate(diags)) {
            return diags;
        }
        await new Promise((r) => setTimeout(r, 200));
    }
    throw new Error(`Timed out waiting for diagnostics on ${uri.toString()}`);
}

suite("Filament Material LSP integration", () => {
    test("valid .mat produces no error diagnostics", async () => {
        const doc = await openDocument("simple.mat");
        await waitForDiagnostics(doc.uri, (d) => d.length >= 0);
        // Give the server a moment to publish, then assert no errors.
        await new Promise((r) => setTimeout(r, 800));
        const errors = vscode.languages
            .getDiagnostics(doc.uri)
            .filter((d) => d.severity === vscode.DiagnosticSeverity.Error);
        assert.strictEqual(
            errors.length,
            0,
            `Expected no errors on valid file, got: ${JSON.stringify(errors)}`
        );
    });

    test("invalid .mat produces a diagnostic", async () => {
        const doc = await openDocument("invalid.mat");
        const diags = await waitForDiagnostics(doc.uri, (d) => d.length > 0);
        assert.ok(diags.length > 0, "Expected at least one diagnostic on invalid.mat");
    });

    test("completion returns material keywords", async () => {
        const doc = await openDocument("simple.mat");
        const result = (await vscode.commands.executeCommand(
            "vscode.executeCompletionItemProvider",
            doc.uri,
            new vscode.Position(1, 10),
            ""
        )) as vscode.CompletionList;
        const items = result.items ?? [];
        assert.ok(items.length > 0, "Expected completion items");
        const labels = items.map((i) => (typeof i.label === "string" ? i.label : i.label.label));
        assert.ok(
            labels.includes("shadingModel") || labels.includes("name"),
            `Expected material keywords, got: ${labels.join(", ")}`
        );
    });

    test("hover returns documentation for a shader symbol", async () => {
        const doc = await openDocument("simple.mat");
        const result = (await vscode.commands.executeCommand(
            "vscode.executeHoverProvider",
            doc.uri,
            new vscode.Position(13, 26)
        )) as vscode.Hover[];
        assert.ok(result.length > 0, "Expected hover content on MaterialInputs");
        const text = result
            .flatMap((h) => (Array.isArray(h.contents) ? h.contents : [h.contents]))
            .map((c) =>
                typeof c === "string" ? c : (c as vscode.MarkdownString).value ?? ""
            )
            .join(" ");
        assert.ok(
            /shader input struct/i.test(text),
            `Expected MaterialInputs doc, got: ${text}`
        );
    });
});
