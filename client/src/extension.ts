import * as path from "path";
import { workspace, ExtensionContext } from "vscode";

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
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
