import express from "express";
import { glob } from "glob";
import * as path from "path";
import * as fs from "fs";

const rootDirectory = process.env.ROOT_DIRECTORY
    ? path.resolve(process.env.ROOT_DIRECTORY)
    : undefined;
if (!rootDirectory) {
    throw new Error("Please set ROOT_DIRECTORY");
}

const app = express();

app.get("/api/files", async (req, res) => {
    try {
        const filenames = await glob("**/*", {
            cwd: rootDirectory,
        });

        const filenamesWithMetadata = await Promise.all(
            filenames.map(async (filename) => ({
                filename,
                size: await fs.promises
                    .stat(path.resolve(rootDirectory, filename))
                    .then((stat) => stat.size),
            }))
        );

        res.json(filenamesWithMetadata);
    } catch (err) {
        console.error(err);
        res.status(500).end("failed to list files");
    }
});

app.get("/api/file", async (req, res) => {
    try {
        const filename = req.query.filename;
        if (typeof filename !== "string") {
            res.status(400).end("invalid filename parameter");
            return;
        }

        const filepath = path.resolve(rootDirectory, filename);
        if (!filepath.startsWith(rootDirectory + "/")) {
            res.status(400).end("invalid filename parameter");
        }

        const file = await fs.promises.readFile(filepath, {
            encoding: "utf-8",
        });
        res.header("Content-Type", "application/json").end(file);
    } catch (err) {
        console.error(err);
        res.status(500).end("failed to get files");
    }
});

app.listen(3000);
