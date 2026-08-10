import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { spawn, SpawnOptions } from "child_process";
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

/** Expose the active LanguageClient for integration tests. */
export function getClient(): LanguageClient | undefined {
    return client;
}

export async function activate(context: ExtensionContext) {
    // Resolve the language server binary. If it cannot be located, surface a
    // clear, actionable error instead of failing silently.
    let serverPath: string;
    try {
        serverPath = await ensureServerBinary(context);
    } catch (err) {
        const detail = err instanceof Error ? err.message : String(err);
        window.showErrorMessage(
            `Filament Material LSP could not start: ${detail}\n\n` +
                "Either build the Rust server (npm run compile:rust) or install a released version of the extension. " +
                "The language server feature will be unavailable until the binary is present."
        );
        return;
    }

    const matcConfig = workspace.getConfiguration("filamentMat");

    const serverOptions: ServerOptions = {
        command: serverPath,
        args: [],
        transport: TransportKind.stdio,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "filament-mat" }],
        initializationOptions: {
            matc: {
                matcPath: matcConfig.get<string>("matcPath", "matc"),
                matcPlatform: matcConfig.get<string>("matcPlatform", "desktop"),
                matcApi: matcConfig.get<string>("matcApi", "vulkan"),
            },
        },
    };

    client = new LanguageClient(
        "filamentMatLsp",
        "Filament Material Language Server",
        serverOptions,
        clientOptions
    );

    client.start();

    context.subscriptions.push(
        commands.registerCommand("filamentMat.compile", async () => {
            await compileMaterial();
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

/**
 * Compile the active .mat file with `matc`, streaming output to a dedicated
 * output channel. If matc cannot be found, degrade gracefully with guidance
 * instead of launching a broken terminal.
 */
async function compileMaterial(): Promise<void> {
    const editor = window.activeTextEditor;
    if (!editor || editor.document.languageId !== "filament-mat") {
        window.showWarningMessage("No .mat file is currently active");
        return;
    }

    const matcPath = workspace
        .getConfiguration("filamentMat")
        .get<string>("matcPath", "matc");

    // Resolve matc. A bare "matc" falls back to PATH resolution.
    const resolvedMatc = resolveMatc(matcPath);
    if (!resolvedMatc) {
        window.showErrorMessage(
            `matc was not found at "${matcPath}".\n\n` +
                "Filament's material compiler (matc) is required to compile. " +
                "Install Filament or set the correct path in the " +
                '"Filament Material > Matc Path" setting.'
        );
        return;
    }

    const filePath = editor.document.uri.fsPath;
    const outputPath = filePath.replace(/\.mat$/i, ".filamat");

    const channel = window.createOutputChannel("Filament Material Compiler");
    channel.show(true);
    channel.appendLine(
        `$ ${resolvedMatc} -o "${outputPath}" "${filePath}"`
    );

    const spawnOpts: SpawnOptions = { cwd: path.dirname(filePath) };
    const child = spawn(resolvedMatc, ["-o", outputPath, filePath], spawnOpts);

    child.stdout?.on("data", (d) => channel.append(d.toString()));
    child.stderr?.on("data", (d) => channel.append(d.toString()));

    child.on("error", (err) => {
        channel.appendLine(`\nFailed to launch matc: ${err.message}`);
        window.showErrorMessage(`Failed to launch matc: ${err.message}`);
    });

    child.on("close", (code) => {
        if (code === 0) {
            channel.appendLine(`\n✔ Compiled successfully: ${path.basename(outputPath)}`);
            window.showInformationMessage(
                `Compiled ${path.basename(filePath)} successfully`
            );
        } else {
            channel.appendLine(`\n✖ Compilation failed with exit code ${code}`);
            window.showErrorMessage(
                `Compilation failed with exit code ${code}. See the 'Filament Material Compiler' output.`
            );
        }
    });
}

/**
 * Resolve the matc executable. Empty string, "matc", or true PATH lookup all
 * resolve via PATH; otherwise the configured path is checked for existence.
 */
function resolveMatc(matcPath: string): string | undefined {
    const trimmed = matcPath.trim();
    if (trimmed === "" || trimmed === "matc") {
        return "matc";
    }
    if (fs.existsSync(trimmed)) {
        return trimmed;
    }
    // On Windows, expand a bare name against PATH.
    if (os.platform() === "win32") {
        const onPath = findOnPath(trimmed);
        if (onPath) return onPath;
    }
    return undefined;
}

function findOnPath(name: string): string | undefined {
    const pathVar = process.env.PATH || "";
    const candidates = pathVar
        .split(path.delimiter)
        .filter((p) => p.length > 0)
        .map((p) => path.join(p, name))
        .filter((p) => fs.existsSync(p));
    return candidates[0];
}

export async function deactivate(): Promise<void> {
    if (client) {
        await client.stop();
    }
}
