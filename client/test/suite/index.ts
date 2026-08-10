import * as fs from "fs";
import * as path from "path";
import Mocha = require("mocha");

export async function run(): Promise<void> {
    const mocha = new Mocha({
        ui: "tdd",
        color: true,
        timeout: 60000,
    });

    const testsRoot = __dirname;
    const files = fs
        .readdirSync(testsRoot)
        .filter((f) => f.endsWith(".test.js"));
    files.forEach((f) => mocha.addFile(path.resolve(testsRoot, f)));

    try {
        await new Promise<void>((resolve, reject) => {
            mocha.run((failures) => {
                if (failures > 0) {
                    reject(new Error(`${failures} tests failed.`));
                } else {
                    resolve();
                }
            });
        });
    } catch (err) {
        console.error(err);
        throw err;
    }
}