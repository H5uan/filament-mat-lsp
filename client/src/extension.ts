import * as path from "path";
import * as vscode from "vscode";
import { workspace, ExtensionContext, commands, window, Uri } from "vscode";

import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from "vscode-languageclient/node";

import { ensureServerBinary } from "./download";

let client: LanguageClient;

export async function activate(context: ExtensionContext) {
    const serverPath = await ensureServerBinary(context);

    const serverOptions: ServerOptions = {
        command: serverPath,
        args: [],
        transport: TransportKind.stdio,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "filament-mat" }],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher("**/.clientrc"),
        },
    };

    client = new LanguageClient(
        "filamentMatLsp",
        "Filament Material Language Server",
        serverOptions,
        clientOptions
    );

    client.start();

    // Register custom commands
    context.subscriptions.push(
        commands.registerCommand("filamentMat.compile", async () => {
            const editor = window.activeTextEditor;
            if (!editor || editor.document.languageId !== "filament-mat") {
                window.showWarningMessage("No .mat file is currently active");
                return;
            }

            const matcPath = workspace
                .getConfiguration("filamentMat")
                .get<string>("matcPath", "matc");
            const filePath = editor.document.uri.fsPath;
            const outputPath = filePath.replace(/\.mat$/, ".filamat");

            const terminal =
                window.activeTerminal ||
                window.createTerminal("Filament Material Compiler");
            terminal.show();
            terminal.sendText(
                `${matcPath} -o "${outputPath}" "${filePath}"`
            );
            window.showInformationMessage(
                `Compiling ${path.basename(filePath)}...`
            );
        })
    );

    context.subscriptions.push(
        commands.registerCommand("filamentMat.showDocumentation", async () => {
            const editor = window.activeTextEditor;
            if (!editor || editor.document.languageId !== "filament-mat") {
                window.showWarningMessage("No .mat file is currently active");
                return;
            }

            const position = editor.selection.active;
            const wordRange = editor.document.getWordRangeAtPosition(position);
            const word = wordRange
                ? editor.document.getText(wordRange)
                : "";

            if (!word) {
                window.showInformationMessage(
                    "No symbol found at cursor position"
                );
                return;
            }

            // Open Filament documentation for the symbol
            const docUrl = `https://google.github.io/filament/Materials.html#${word.toLowerCase()}`;
            await vscode.env.openExternal(Uri.parse(docUrl));
        })
    );
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
