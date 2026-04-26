import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';

suite('Filament Material LSP E2E Test Suite', () => {
  let doc: vscode.TextDocument;
  let editor: vscode.TextEditor;

  setup(async () => {
    const testFile = path.join(
      __dirname,
      '../../../..',
      'test/simple.mat'
    );
    doc = await vscode.workspace.openTextDocument(testFile);
    editor = await vscode.window.showTextDocument(doc);
  });

  test('Extension should be active for .mat files', async () => {
    const ext = vscode.extensions.getExtension('undefined_publisher.filament-mat-lsp');
    assert.ok(ext, 'Extension should be found');
    await ext?.activate();
    assert.ok(ext?.isActive, 'Extension should be active');
  });

  test('Should provide completions for material properties', async () => {
    const position = new vscode.Position(2, 4); // Inside material block
    const completions = await vscode.commands.executeCommand<
      vscode.CompletionList
    >('vscode.executeCompletionItemProvider', doc.uri, position);

    assert.ok(completions, 'Should return completions');
    assert.ok(
      completions!.items.length > 0,
      'Should have completion items'
    );
    
    const labels = completions!.items.map((item) => item.label);
    assert.ok(
      labels.includes('shadingModel'),
      'Should include shadingModel'
    );
    assert.ok(labels.includes('blending'), 'Should include blending');
  });

  test('Should provide hover for material properties', async () => {
    const position = new vscode.Position(3, 8); // On "shadingModel"
    const hover = await vscode.commands.executeCommand<
      vscode.Hover[]
    >('vscode.executeHoverProvider', doc.uri, position);

    assert.ok(hover && hover.length > 0, 'Should return hover');
  });

  test('Should provide document symbols', async () => {
    const symbols = await vscode.commands.executeCommand<
      vscode.DocumentSymbol[]
    >('vscode.executeDocumentSymbolProvider', doc.uri);

    assert.ok(symbols, 'Should return symbols');
    assert.ok(symbols!.length > 0, 'Should have document symbols');
  });

  test('Should provide diagnostics for missing required properties', async () => {
    // Wait a bit for diagnostics to be computed
    await new Promise((resolve) => setTimeout(resolve, 2000));

    const diagnostics = vscode.languages.getDiagnostics(doc.uri);
    assert.ok(diagnostics, 'Should have diagnostics');
  });
});
