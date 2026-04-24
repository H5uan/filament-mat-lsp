import * as https from "https";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import { ExtensionContext } from "vscode";

function getTargetTriple(): string {
    const platform = os.platform();
    const arch = os.arch();
    if (platform === "win32" && arch === "x64") return "x86_64-pc-windows-msvc";
    if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
    if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
    if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-gnu";
    throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

function downloadFile(url: string, dest: string): Promise<void> {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(dest);
        https
            .get(url, (response) => {
                if (response.statusCode === 302 || response.statusCode === 301) {
                    if (response.headers.location) {
                        downloadFile(response.headers.location, dest)
                            .then(resolve)
                            .catch(reject);
                        return;
                    }
                }
                if (response.statusCode !== 200) {
                    reject(new Error(`Download failed with status ${response.statusCode}`));
                    return;
                }
                response.pipe(file);
                file.on("finish", () => {
                    file.close();
                    resolve();
                });
            })
            .on("error", (err) => {
                fs.unlink(dest, () => {});
                reject(err);
            });
    });
}

function getWorkspaceBinaryPath(context: ExtensionContext): string | undefined {
    const binaryName =
        os.platform() === "win32" ? "filament-mat-lsp.exe" : "filament-mat-lsp";
    const workspaceBinary = path.join(
        context.extensionPath,
        "native",
        "target",
        "release",
        binaryName
    );
    if (fs.existsSync(workspaceBinary)) {
        return workspaceBinary;
    }
    return undefined;
}

export async function ensureServerBinary(
    context: ExtensionContext
): Promise<string> {
    const target = getTargetTriple();
    const binaryName =
        os.platform() === "win32" ? "filament-mat-lsp.exe" : "filament-mat-lsp";
    const binaryDir = path.join(context.globalStorageUri.fsPath, "server");
    const binaryPath = path.join(binaryDir, binaryName);

    // For development: always prefer local workspace binary
    const workspaceBinary = getWorkspaceBinaryPath(context);
    if (workspaceBinary) {
        return workspaceBinary;
    }

    if (fs.existsSync(binaryPath)) {
        return binaryPath;
    }

    fs.mkdirSync(binaryDir, { recursive: true });

    // Download from GitHub Releases
    const version = "0.0.1"; // Should match package.json version
    const ext = os.platform() === "win32" ? ".exe" : "";
    const url = `https://github.com/H5uan/filament-mat-lsp/releases/download/v${version}/filament-mat-lsp-${target}${ext}`;

    await downloadFile(url, binaryPath);
    fs.chmodSync(binaryPath, "755");

    return binaryPath;
}
