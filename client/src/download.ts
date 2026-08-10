import * as https from "https";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import { ExtensionContext } from "vscode";

function getTargetTriple(): string {
    const platform = os.platform();
    const arch = os.arch();
    if (platform === "win32" && arch === "x64") return "x86_64-pc-windows-msvc";
    if (platform === "win32" && arch === "arm64") return "aarch64-pc-windows-msvc";
    if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
    if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
    if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-gnu";
    if (platform === "linux" && arch === "arm64") return "aarch64-unknown-linux-gnu";
    throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

function downloadFile(url: string, dest: string, timeoutMs: number = 30000): Promise<void> {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(dest);
        const req = https
            .get(url, { timeout: timeoutMs }, (response) => {
                if (response.statusCode === 302 || response.statusCode === 301 || response.statusCode === 307 || response.statusCode === 308) {
                    if (response.headers.location) {
                        downloadFile(response.headers.location, dest, timeoutMs)
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
        req.setTimeout(timeoutMs, () => {
            req.destroy();
            fs.unlink(dest, () => {});
            reject(new Error(`Download timed out after ${timeoutMs}ms`));
        });
    });
}

async function downloadWithRetry(url: string, dest: string, retries: number = 3): Promise<void> {
    let lastError: Error | undefined;
    for (let i = 0; i < retries; i++) {
        try {
            await downloadFile(url, dest);
            return;
        } catch (err) {
            lastError = err instanceof Error ? err : new Error(String(err));
            if (i < retries - 1) {
                const delay = Math.pow(2, i) * 1000;
                await new Promise((resolve) => setTimeout(resolve, delay));
            }
        }
    }
    throw lastError || new Error("Download failed after retries");
}

function getBundledBinaryPath(context: ExtensionContext): string | undefined {
    const binaryName =
        os.platform() === "win32" ? "filament-mat-lsp.exe" : "filament-mat-lsp";
    const bundledBinary = path.join(
        context.extensionPath,
        "native",
        "bin",
        getTargetTriple(),
        binaryName
    );
    if (fs.existsSync(bundledBinary)) {
        return bundledBinary;
    }
    return undefined;
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

function getPackageVersion(): string {
    try {
        const packageJsonPath = path.join(__dirname, "..", "..", "package.json");
        const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
        return packageJson.version || "0.0.1";
    } catch {
        return "0.0.1";
    }
}

export async function ensureServerBinary(
    context: ExtensionContext
): Promise<string> {
    const target = getTargetTriple();
    const binaryName =
        os.platform() === "win32" ? "filament-mat-lsp.exe" : "filament-mat-lsp";
    const binaryDir = path.join(context.globalStorageUri.fsPath, "server");
    const binaryPath = path.join(binaryDir, binaryName);

    // 1. Bundled binary shipped inside the .vsix (primary path for packaged
    //    extensions; deterministic, no network required).
    const bundledBinary = getBundledBinaryPath(context);
    if (bundledBinary) {
        return bundledBinary;
    }

    // 2. Local workspace release build (development machine).
    const workspaceBinary = getWorkspaceBinaryPath(context);
    if (workspaceBinary) {
        return workspaceBinary;
    }

    // 3. Previously downloaded binary cached in global storage.
    if (fs.existsSync(binaryPath)) {
        return binaryPath;
    }

    fs.mkdirSync(binaryDir, { recursive: true });

    // Download from GitHub Releases
    const version = getPackageVersion();
    const ext = os.platform() === "win32" ? ".exe" : "";
    const url = `https://github.com/H5uan/filament-mat-lsp/releases/download/v${version}/filament-mat-lsp-${target}${ext}`;

    await downloadWithRetry(url, binaryPath);
    if (os.platform() !== "win32") {
        fs.chmodSync(binaryPath, "755");
    }

    return binaryPath;
}
