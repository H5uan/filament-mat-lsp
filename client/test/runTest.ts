import * as path from "path";
import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
    // Extension root is the repository root (parent of client/).
    const extensionDevelopmentPath = path.resolve(__dirname, "..", "..");
    const extensionTestsPath = path.resolve(__dirname, "suite");

    await runTests({
        extensionDevelopmentPath,
        extensionTestsPath,
        launchArgs: [],
    });
}

main().catch((err) => {
    console.error("Failed to run VS Code integration tests");
    console.error(err);
    process.exit(1);
});